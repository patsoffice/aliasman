use std::cmp::Ordering;
use std::fmt;

use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Alias {
    pub alias: String,
    pub domain: String,
    pub email_addresses: Vec<String>,
    pub description: String,
    pub suspended: bool,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub suspended_at: Option<DateTime<Utc>>,
}

impl Alias {
    /// Returns "alias@domain" as a unique key.
    pub fn key(&self) -> String {
        format!("{}@{}", self.alias, self.domain)
    }

    /// Returns "alias@domain" for display purposes.
    pub fn full_alias(&self) -> String {
        self.key()
    }

    /// Returns true if this alias matches the given filter.
    pub fn matches(&self, filter: &AliasFilter) -> bool {
        if filter.exclude_suspended && self.suspended {
            return false;
        }
        if filter.exclude_enabled && !self.suspended {
            return false;
        }

        let checks: Vec<bool> = [
            filter.alias.as_ref().map(|r| r.is_match(&self.alias)),
            filter.domain.as_ref().map(|r| r.is_match(&self.domain)),
            filter
                .email_address
                .as_ref()
                .map(|r| self.email_addresses.iter().any(|addr| r.is_match(addr))),
            filter
                .description
                .as_ref()
                .map(|r| r.is_match(&self.description)),
        ]
        .into_iter()
        .flatten()
        .collect();

        if checks.is_empty() {
            return true;
        }

        // Match any: at least one regex matches
        checks.iter().any(|&matched| matched)
    }
}

impl fmt::Display for Alias {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}@{} -> {}",
            self.alias,
            self.domain,
            self.email_addresses.join(", ")
        )
    }
}

impl Ord for Alias {
    fn cmp(&self, other: &Self) -> Ordering {
        self.domain
            .cmp(&other.domain)
            .then_with(|| self.alias.cmp(&other.alias))
    }
}

impl PartialOrd for Alias {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Default)]
pub struct AliasFilter {
    pub alias: Option<Regex>,
    pub domain: Option<Regex>,
    pub email_address: Option<Regex>,
    pub description: Option<Regex>,
    pub exclude_suspended: bool,
    pub exclude_enabled: bool,
}

/// Generate a random hex alias string of the specified length (max 32).
pub fn generate_random_alias(length: usize) -> String {
    use rand::Rng;

    let length = length.min(32);
    let mut rng = rand::rng();
    let bytes: Vec<u8> = (0..length.div_ceil(2))
        .map(|_| rng.random::<u8>())
        .collect();
    let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    hex[..length].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_alias_key() {
        let a = sample_alias();
        assert_eq!(a.key(), "test123@example.com");
    }

    #[test]
    fn test_alias_display() {
        let a = sample_alias();
        assert_eq!(format!("{}", a), "test123@example.com -> user@example.com");
    }

    #[test]
    fn test_alias_ordering() {
        let mut aliases = [
            Alias {
                domain: "z.com".to_string(),
                alias: "a".to_string(),
                ..sample_alias()
            },
            Alias {
                domain: "a.com".to_string(),
                alias: "z".to_string(),
                ..sample_alias()
            },
            Alias {
                domain: "a.com".to_string(),
                alias: "a".to_string(),
                ..sample_alias()
            },
        ];
        aliases.sort();
        assert_eq!(aliases[0].key(), "a@a.com");
        assert_eq!(aliases[1].key(), "z@a.com");
        assert_eq!(aliases[2].key(), "a@z.com");
    }

    #[test]
    fn test_filter_empty_matches_all() {
        let a = sample_alias();
        let filter = AliasFilter::default();
        assert!(a.matches(&filter));
    }

    #[test]
    fn test_filter_exclude_suspended() {
        let mut a = sample_alias();
        a.suspended = true;
        let filter = AliasFilter {
            exclude_suspended: true,
            ..Default::default()
        };
        assert!(!a.matches(&filter));
    }

    #[test]
    fn test_filter_exclude_enabled() {
        let a = sample_alias();
        let filter = AliasFilter {
            exclude_enabled: true,
            ..Default::default()
        };
        assert!(!a.matches(&filter));
    }

    #[test]
    fn test_filter_by_description() {
        let a = sample_alias();
        let filter = AliasFilter {
            description: Some(Regex::new("Test").unwrap()),
            ..Default::default()
        };
        assert!(a.matches(&filter));

        let filter_no_match = AliasFilter {
            description: Some(Regex::new("Nope").unwrap()),
            ..Default::default()
        };
        assert!(!a.matches(&filter_no_match));
    }

    #[test]
    fn test_generate_random_alias_length() {
        let alias = generate_random_alias(16);
        assert_eq!(alias.len(), 16);
        assert!(alias.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_random_alias_max() {
        let alias = generate_random_alias(64);
        assert_eq!(alias.len(), 32);
    }
}
