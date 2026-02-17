use async_trait::async_trait;
use aws_sdk_s3::config::Region;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::Object;
use aws_sdk_s3::Client;
use chrono::{DateTime, Utc};
use md5::{Digest, Md5};
use std::collections::HashMap;
use tokio::sync::RwLock;

use crate::error::{Error, Result};
use crate::model::{Alias, AliasFilter};
use crate::storage::StorageProvider;

/// New Rust-native S3 format - JSON body with proper nulls and Rust field naming
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct S3Alias {
    alias: String,
    domain: String,
    #[serde(rename = "email_addresses")]
    email_addresses: Vec<String>,
    description: String,
    suspended: bool,
    #[serde(with = "rfc3339_secs")]
    created_at: DateTime<Utc>,
    #[serde(with = "rfc3339_secs")]
    modified_at: DateTime<Utc>,
    #[serde(with = "rfc3339_secs_option")]
    suspended_at: Option<DateTime<Utc>>,
}

impl S3Alias {
    fn from_alias(alias: &Alias) -> Self {
        Self {
            alias: alias.alias.clone(),
            domain: alias.domain.clone(),
            email_addresses: alias.email_addresses.clone(),
            description: alias.description.clone(),
            suspended: alias.suspended,
            created_at: alias.created_at,
            modified_at: alias.modified_at,
            suspended_at: alias.suspended_at,
        }
    }

    fn into_alias(self) -> Alias {
        Alias {
            alias: self.alias,
            domain: self.domain,
            email_addresses: self.email_addresses,
            description: self.description,
            suspended: self.suspended,
            created_at: self.created_at,
            modified_at: self.modified_at,
            suspended_at: self.suspended_at,
        }
    }
}

/// Legacy Go S3 format - used for reading old format during conversion
#[derive(Debug, serde::Deserialize)]
struct LegacyS3Alias {
    alias: String,
    domain: String,
    #[serde(rename = "email_addresses")]
    email_addresses: Vec<String>,
    description: String,
    suspended: bool,
    #[serde(rename = "created_ts")]
    created_ts: String,
    #[serde(rename = "modified_ts")]
    modified_ts: String,
    #[serde(rename = "suspended_ts")]
    suspended_ts: String,
}

impl LegacyS3Alias {
    fn into_alias(self) -> Result<Alias> {
        let created_at = parse_rfc3339(&self.created_ts)?;
        let modified_at = parse_rfc3339(&self.modified_ts)?;
        let suspended_at = parse_go_zero_time(&self.suspended_ts)?;

        Ok(Alias {
            alias: self.alias,
            domain: self.domain,
            email_addresses: self.email_addresses,
            description: self.description,
            suspended: self.suspended,
            created_at,
            modified_at,
            suspended_at,
        })
    }
}

/// Serde module for RFC3339 timestamps with second precision
mod rfc3339_secs {
    use chrono::{DateTime, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = date.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        DateTime::parse_from_rfc3339(&s)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(serde::de::Error::custom)
    }
}

/// Serde module for optional RFC3339 timestamps
mod rfc3339_secs_option {
    use chrono::{DateTime, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match date {
            Some(d) => {
                let s = d.format("%Y-%m-%dT%H:%M:%SZ").to_string();
                serializer.serialize_str(&s)
            }
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt: Option<String> = Option::deserialize(deserializer)?;
        match opt {
            Some(s) => DateTime::parse_from_rfc3339(&s)
                .map(|dt| Some(dt.with_timezone(&Utc)))
                .map_err(serde::de::Error::custom),
            None => Ok(None),
        }
    }
}

fn parse_rfc3339(s: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| Error::Storage(format!("invalid RFC3339 timestamp '{}': {}", s, e).into()))
}

fn parse_go_zero_time(s: &str) -> Result<Option<DateTime<Utc>>> {
    // Go's zero time is "0001-01-01T00:00:00Z"
    if s == "0001-01-01T00:00:00Z" || s.starts_with("0001-01-01") {
        return Ok(None);
    }
    DateTime::parse_from_rfc3339(s)
        .map(|dt| Some(dt.with_timezone(&Utc)))
        .map_err(|e| Error::Storage(format!("invalid RFC3339 timestamp '{}': {}", s, e).into()))
}

fn alias_s3_key(alias: &str, domain: &str) -> String {
    format!("alias-{}@{}", alias, domain)
}

fn parse_alias_from_metadata(metadata: &HashMap<String, String>) -> Result<Alias> {
    let alias = metadata
        .get("alias")
        .ok_or_else(|| Error::Storage("missing 'alias' metadata".into()))?;
    let domain = metadata
        .get("domain")
        .ok_or_else(|| Error::Storage("missing 'domain' metadata".into()))?;
    let email_addresses_str = metadata
        .get("email_addresses")
        .ok_or_else(|| Error::Storage("missing 'email_addresses' metadata".into()))?;
    let description = metadata
        .get("description")
        .ok_or_else(|| Error::Storage("missing 'description' metadata".into()))?;
    let suspended_str = metadata
        .get("suspended")
        .ok_or_else(|| Error::Storage("missing 'suspended' metadata".into()))?;
    let created_ts = metadata
        .get("created_ts")
        .ok_or_else(|| Error::Storage("missing 'created_ts' metadata".into()))?;
    let modified_ts = metadata
        .get("modified_ts")
        .ok_or_else(|| Error::Storage("missing 'modified_ts' metadata".into()))?;
    let suspended_ts = metadata
        .get("suspended_ts")
        .ok_or_else(|| Error::Storage("missing 'suspended_ts' metadata".into()))?;

    let email_addresses: Vec<String> = email_addresses_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let suspended = suspended_str == "true";
    let created_at = parse_rfc3339(created_ts)?;
    let modified_at = parse_rfc3339(modified_ts)?;
    let suspended_at = parse_go_zero_time(suspended_ts)?;

    Ok(Alias {
        alias: alias.clone(),
        domain: domain.clone(),
        email_addresses,
        description: description.clone(),
        suspended,
        created_at,
        modified_at,
        suspended_at,
    })
}

fn s3_err<E: std::error::Error + Send + Sync + 'static>(e: E) -> Error {
    Error::Storage(Box::new(e))
}

pub struct S3Storage {
    bucket: String,
    region: Option<String>,
    endpoint: Option<String>,
    access_key_id: Option<String>,
    secret_access_key: Option<String>,
    client: Option<Client>,
    aliases: RwLock<HashMap<String, Alias>>, // key = "alias@domain"
    index_etag: RwLock<Option<String>>,
    read_only: bool,
    legacy_mode: bool, // when true, read Go format (read-only)
}

impl S3Storage {
    pub fn new(
        bucket: impl Into<String>,
        region: Option<String>,
        endpoint: Option<String>,
        access_key_id: Option<String>,
        secret_access_key: Option<String>,
        legacy_mode: bool,
    ) -> Self {
        Self {
            bucket: bucket.into(),
            region,
            endpoint,
            access_key_id,
            secret_access_key,
            client: None,
            aliases: RwLock::new(HashMap::new()),
            index_etag: RwLock::new(None),
            read_only: false,
            legacy_mode,
        }
    }

    async fn build_s3_client(&self) -> Result<Client> {
        let region = self
            .region
            .clone()
            .map(Region::new)
            .unwrap_or_else(|| Region::new("us-east-1"));

        let mut config_builder = aws_sdk_s3::Config::builder()
            .behavior_version_latest()
            .region(region);

        // Set endpoint if provided (for MinIO, LocalStack, etc.)
        if let Some(endpoint) = &self.endpoint {
            config_builder = config_builder.endpoint_url(endpoint).force_path_style(true);
        }

        // Use static credentials if provided, otherwise use default credential chain
        if let (Some(access_key), Some(secret_key)) = (&self.access_key_id, &self.secret_access_key)
        {
            let credentials = aws_sdk_s3::config::Credentials::new(
                access_key.clone(),
                secret_key.clone(),
                None,
                None,
                "static",
            );
            config_builder = config_builder.credentials_provider(credentials);
        } else {
            // Load from standard AWS credential chain
            let sdk_config = aws_config::load_from_env().await;
            let credentials_provider = sdk_config
                .credentials_provider()
                .ok_or_else(|| Error::Config("no AWS credentials found (check ~/.aws/credentials, environment variables, or instance role)".to_string()))?;
            config_builder = config_builder.credentials_provider(credentials_provider);
        }

        let config = config_builder.build();
        Ok(Client::from_conf(config))
    }

    fn client(&self) -> Result<&Client> {
        self.client
            .as_ref()
            .ok_or_else(|| Error::Storage("S3 client not initialized".into()))
    }

    /// Load all aliases from S3 into the in-memory cache.
    /// Called by both `open()` and `refresh()`. Uses `&self` because
    /// it writes through the `RwLock` fields.
    async fn load_aliases(&self) -> Result<()> {
        let client = self.client()?;
        let bucket = self.bucket.clone();

        // List all objects in the bucket
        let mut objects: Vec<Object> = Vec::new();
        let mut continuation_token: Option<String> = None;

        loop {
            let mut request = client.list_objects_v2().bucket(&bucket).max_keys(1000);

            if let Some(token) = &continuation_token {
                request = request.continuation_token(token);
            }

            let result = request.send().await.map_err(s3_err)?;

            if let Some(contents) = result.contents {
                objects.extend(contents);
            }

            if result.is_truncated.unwrap_or(false) {
                continuation_token = result.next_continuation_token;
            } else {
                break;
            }
        }

        // Separate index from alias keys
        let index_object = objects
            .iter()
            .find(|o| o.key.as_ref().map(|k| k == "index").unwrap_or(false));
        let alias_objects: Vec<&Object> = objects
            .iter()
            .filter(|o| {
                o.key
                    .as_ref()
                    .map(|k| k.starts_with("alias-"))
                    .unwrap_or(false)
            })
            .collect();

        // Check if we can use the fast path via index
        let alias_count = alias_objects.len();
        let mut loaded_from_index = false;

        if let Some(index) = index_object {
            if alias_count > 0 {
                // Check if index is newer than all alias objects
                let index_modified = index.last_modified.as_ref();
                let all_aliases_older = alias_objects.iter().all(|o| {
                    o.last_modified
                        .as_ref()
                        .is_none_or(|t| index_modified.is_none_or(|idx| t <= idx))
                });

                if all_aliases_older {
                    // Try to load from index
                    match client
                        .get_object()
                        .bucket(&bucket)
                        .key("index")
                        .send()
                        .await
                    {
                        Ok(result) => {
                            let body = result.body.collect().await.map_err(s3_err)?;
                            let bytes = body.into_bytes();

                            if self.legacy_mode {
                                // Parse legacy format
                                let legacy_aliases: Vec<LegacyS3Alias> =
                                    serde_json::from_slice(&bytes)?;
                                if legacy_aliases.len() == alias_count {
                                    let mut aliases = HashMap::new();
                                    for la in legacy_aliases {
                                        let alias = la.into_alias()?;
                                        let key = format!("{}@{}", alias.alias, alias.domain);
                                        aliases.insert(key, alias);
                                    }
                                    *self.aliases.write().await = aliases;
                                    *self.index_etag.write().await =
                                        result.e_tag.map(|s| s.trim_matches('"').to_string());
                                    loaded_from_index = true;
                                }
                            } else {
                                // Parse new format
                                let s3_aliases: Vec<S3Alias> = serde_json::from_slice(&bytes)?;
                                if s3_aliases.len() == alias_count {
                                    let mut aliases = HashMap::new();
                                    for sa in s3_aliases {
                                        let alias = sa.into_alias();
                                        let key = format!("{}@{}", alias.alias, alias.domain);
                                        aliases.insert(key, alias);
                                    }
                                    *self.aliases.write().await = aliases;
                                    *self.index_etag.write().await =
                                        result.e_tag.map(|s| s.trim_matches('"').to_string());
                                    loaded_from_index = true;
                                }
                            }
                        }
                        Err(e) => {
                            // Log warning but continue with slow path
                            eprintln!("Warning: Failed to load index: {}", e);
                        }
                    }
                }
            }
        }

        if !loaded_from_index {
            // Slow path: load each alias object individually
            let mut aliases = HashMap::new();

            if self.legacy_mode {
                // Use HeadObject for legacy format (metadata stored in headers)
                for obj in alias_objects {
                    if let Some(key) = obj.key.as_ref() {
                        match client.head_object().bucket(&bucket).key(key).send().await {
                            Ok(head_result) => {
                                let metadata: HashMap<String, String> = head_result
                                    .metadata
                                    .unwrap_or_default()
                                    .into_iter()
                                    .map(|(k, v)| (k.to_lowercase(), v))
                                    .collect();

                                match parse_alias_from_metadata(&metadata) {
                                    Ok(alias) => {
                                        let map_key = format!("{}@{}", alias.alias, alias.domain);
                                        aliases.insert(map_key, alias);
                                    }
                                    Err(e) => {
                                        eprintln!(
                                            "Warning: Failed to parse metadata for {}: {}",
                                            key, e
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Warning: Failed to head object {}: {}", key, e);
                            }
                        }
                    }
                }
            } else {
                // Use GetObject for new format (data stored in body)
                for obj in alias_objects {
                    if let Some(key) = obj.key.as_ref() {
                        match client.get_object().bucket(&bucket).key(key).send().await {
                            Ok(get_result) => {
                                let body = match get_result.body.collect().await {
                                    Ok(b) => b,
                                    Err(e) => {
                                        eprintln!(
                                            "Warning: Failed to read body for {}: {}",
                                            key, e
                                        );
                                        continue;
                                    }
                                };
                                let bytes = body.into_bytes();

                                match serde_json::from_slice::<S3Alias>(&bytes) {
                                    Ok(s3_alias) => {
                                        let alias = s3_alias.into_alias();
                                        let map_key = format!("{}@{}", alias.alias, alias.domain);
                                        aliases.insert(map_key, alias);
                                    }
                                    Err(e) => {
                                        eprintln!(
                                            "Warning: Failed to parse JSON for {}: {}",
                                            key, e
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Warning: Failed to get object {}: {}", key, e);
                            }
                        }
                    }
                }
            }

            *self.aliases.write().await = aliases;
        }

        Ok(())
    }
}

#[async_trait]
impl StorageProvider for S3Storage {
    async fn open(&mut self, read_only: bool) -> Result<()> {
        self.read_only = read_only;
        self.client = Some(self.build_s3_client().await?);
        self.load_aliases().await
    }

    async fn refresh(&mut self) -> Result<()> {
        if self.client.is_none() {
            return Ok(());
        }
        self.aliases.write().await.clear();
        self.load_aliases().await
    }

    async fn close(&mut self) -> Result<()> {
        if self.read_only || self.legacy_mode {
            return Ok(());
        }

        let client = match self.client() {
            Ok(c) => c,
            Err(_) => return Ok(()), // Client not initialized, nothing to do
        };

        // Get all aliases as a sorted vector
        let aliases = self.aliases.read().await;
        let mut alias_vec: Vec<&Alias> = aliases.values().collect();
        alias_vec.sort_by_key(|a| (&a.domain, &a.alias));

        // Convert to S3Alias format
        let s3_aliases: Vec<S3Alias> = alias_vec.into_iter().map(S3Alias::from_alias).collect();

        // Serialize to JSON
        let json_bytes = serde_json::to_vec_pretty(&s3_aliases)?;

        // Compute MD5
        let mut hasher = Md5::new();
        hasher.update(&json_bytes);
        let md5_digest = format!("{:x}", hasher.finalize());

        // Check if index needs updating
        let current_etag = self.index_etag.read().await;
        if current_etag.as_ref().map(|s| s.as_str()) == Some(&md5_digest) {
            // Index is up to date, skip write
            return Ok(());
        }
        drop(current_etag);

        // Upload index
        client
            .put_object()
            .bucket(&self.bucket)
            .key("index")
            .body(ByteStream::from(json_bytes))
            .content_type("application/json")
            .send()
            .await
            .map_err(s3_err)?;

        Ok(())
    }

    async fn get(&self, alias: &str, domain: &str) -> Result<Option<Alias>> {
        let key = format!("{}@{}", alias, domain);
        let aliases = self.aliases.read().await;
        Ok(aliases.get(&key).cloned())
    }

    async fn put(&self, alias: &Alias) -> Result<()> {
        if self.read_only {
            return Err(Error::Storage("storage is read-only".into()));
        }

        if self.legacy_mode {
            return Err(Error::Storage("cannot write in legacy mode".into()));
        }

        let client = self.client()?;
        let key = alias_s3_key(&alias.alias, &alias.domain);
        let s3_alias = S3Alias::from_alias(alias);
        let json_bytes = serde_json::to_vec(&s3_alias)?;

        client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(ByteStream::from(json_bytes))
            .content_type("application/json")
            .send()
            .await
            .map_err(s3_err)?;

        let map_key = format!("{}@{}", alias.alias, alias.domain);
        self.aliases.write().await.insert(map_key, alias.clone());

        Ok(())
    }

    async fn update(&self, alias: &Alias) -> Result<()> {
        let key = format!("{}@{}", alias.alias, alias.domain);
        let aliases = self.aliases.read().await;
        if !aliases.contains_key(&key) {
            return Err(Error::AliasNotFound {
                alias: alias.alias.clone(),
                domain: alias.domain.clone(),
            });
        }
        drop(aliases);

        let mut updated = alias.clone();
        updated.modified_at = Utc::now();
        self.put(&updated).await
    }

    async fn delete(&self, alias: &str, domain: &str) -> Result<()> {
        if self.read_only {
            return Err(Error::Storage("storage is read-only".into()));
        }

        if self.legacy_mode {
            return Err(Error::Storage("cannot delete in legacy mode".into()));
        }

        let key = format!("{}@{}", alias, domain);
        let aliases = self.aliases.read().await;
        if !aliases.contains_key(&key) {
            return Err(Error::AliasNotFound {
                alias: alias.to_string(),
                domain: domain.to_string(),
            });
        }
        drop(aliases);

        let client = self.client()?;
        let s3_key = alias_s3_key(alias, domain);

        client
            .delete_object()
            .bucket(&self.bucket)
            .key(&s3_key)
            .send()
            .await
            .map_err(s3_err)?;

        self.aliases.write().await.remove(&key);

        Ok(())
    }

    async fn search(&self, filter: &AliasFilter) -> Result<Vec<Alias>> {
        let aliases = self.aliases.read().await;
        let results: Vec<Alias> = aliases
            .values()
            .filter(|a| a.matches(filter))
            .cloned()
            .collect();
        Ok(results)
    }

    async fn suspend(&self, alias: &str, domain: &str) -> Result<()> {
        if self.read_only {
            return Err(Error::Storage("storage is read-only".into()));
        }

        if self.legacy_mode {
            return Err(Error::Storage("cannot suspend in legacy mode".into()));
        }

        let key = format!("{}@{}", alias, domain);
        let aliases = self.aliases.read().await;
        let mut alias_obj = aliases
            .get(&key)
            .cloned()
            .ok_or_else(|| Error::AliasNotFound {
                alias: alias.to_string(),
                domain: domain.to_string(),
            })?;
        drop(aliases);

        let now = Utc::now();
        alias_obj.suspended = true;
        alias_obj.modified_at = now;
        alias_obj.suspended_at = Some(now);

        self.put(&alias_obj).await
    }

    async fn unsuspend(&self, alias: &str, domain: &str) -> Result<()> {
        if self.read_only {
            return Err(Error::Storage("storage is read-only".into()));
        }

        if self.legacy_mode {
            return Err(Error::Storage("cannot unsuspend in legacy mode".into()));
        }

        let key = format!("{}@{}", alias, domain);
        let aliases = self.aliases.read().await;
        let mut alias_obj = aliases
            .get(&key)
            .cloned()
            .ok_or_else(|| Error::AliasNotFound {
                alias: alias.to_string(),
                domain: domain.to_string(),
            })?;
        drop(aliases);

        alias_obj.suspended = false;
        alias_obj.modified_at = Utc::now();
        alias_obj.suspended_at = None;

        self.put(&alias_obj).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_alias() -> Alias {
        Alias {
            alias: "test123".to_string(),
            domain: "example.com".to_string(),
            email_addresses: vec!["user@example.com".to_string()],
            description: "Test alias".to_string(),
            suspended: false,
            created_at: Utc::now(),
            modified_at: Utc::now(),
            suspended_at: None,
        }
    }

    #[test]
    fn test_s3_alias_roundtrip() {
        let alias = sample_alias();
        let s3_alias = S3Alias::from_alias(&alias);
        let converted = s3_alias.into_alias();

        assert_eq!(alias.alias, converted.alias);
        assert_eq!(alias.domain, converted.domain);
        assert_eq!(alias.email_addresses, converted.email_addresses);
        assert_eq!(alias.description, converted.description);
        assert_eq!(alias.suspended, converted.suspended);
    }

    #[test]
    fn test_legacy_alias_conversion() {
        let legacy = LegacyS3Alias {
            alias: "shopping".to_string(),
            domain: "example.com".to_string(),
            email_addresses: vec!["user@example.com".to_string()],
            description: "Shopping sites".to_string(),
            suspended: false,
            created_ts: "2024-01-15T10:00:00Z".to_string(),
            modified_ts: "2024-01-15T10:00:00Z".to_string(),
            suspended_ts: "0001-01-01T00:00:00Z".to_string(),
        };

        let alias = legacy.into_alias().unwrap();
        assert_eq!(alias.alias, "shopping");
        assert_eq!(alias.domain, "example.com");
        assert!(alias.suspended_at.is_none()); // Go zero time becomes None
    }

    #[test]
    fn test_legacy_alias_with_suspended() {
        let legacy = LegacyS3Alias {
            alias: "shopping".to_string(),
            domain: "example.com".to_string(),
            email_addresses: vec!["user@example.com".to_string()],
            description: "Shopping sites".to_string(),
            suspended: true,
            created_ts: "2024-01-15T10:00:00Z".to_string(),
            modified_ts: "2024-01-15T10:00:00Z".to_string(),
            suspended_ts: "2024-01-15T11:00:00Z".to_string(),
        };

        let alias = legacy.into_alias().unwrap();
        assert!(alias.suspended);
        assert!(alias.suspended_at.is_some());
    }

    #[test]
    fn test_alias_s3_key() {
        assert_eq!(
            alias_s3_key("shopping", "example.com"),
            "alias-shopping@example.com"
        );
    }

    #[test]
    fn test_parse_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert("alias".to_string(), "shopping".to_string());
        metadata.insert("domain".to_string(), "example.com".to_string());
        metadata.insert(
            "email_addresses".to_string(),
            "user@example.com,user2@example.com".to_string(),
        );
        metadata.insert("description".to_string(), "Shopping sites".to_string());
        metadata.insert("suspended".to_string(), "false".to_string());
        metadata.insert("created_ts".to_string(), "2024-01-15T10:00:00Z".to_string());
        metadata.insert(
            "modified_ts".to_string(),
            "2024-01-15T10:00:00Z".to_string(),
        );
        metadata.insert(
            "suspended_ts".to_string(),
            "0001-01-01T00:00:00Z".to_string(),
        );

        let alias = parse_alias_from_metadata(&metadata).unwrap();
        assert_eq!(alias.alias, "shopping");
        assert_eq!(alias.email_addresses.len(), 2);
        assert!(alias.suspended_at.is_none());
    }

    #[test]
    fn test_go_zero_time_parsing() {
        assert!(parse_go_zero_time("0001-01-01T00:00:00Z")
            .unwrap()
            .is_none());
        assert!(parse_go_zero_time("0001-01-01T00:00:00+00:00")
            .unwrap()
            .is_none());
        assert!(parse_go_zero_time("2024-01-15T10:00:00Z")
            .unwrap()
            .is_some());
    }
}
