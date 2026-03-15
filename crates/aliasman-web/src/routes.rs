use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse};
use axum::routing::{get, post};
use axum::{Form, Router};
use rust_embed::Embed;
use serde::Deserialize;

use aliasman_core::model::{Alias, AliasFilter};
use aliasman_core::build_alias;

use crate::error::AppError;
use crate::state::SharedState;

// -- Static Assets --

#[derive(Embed)]
#[folder = "static/"]
struct StaticAssets;

// -- View Models --

pub struct AliasView {
    pub full_alias: String,
    pub email_addresses: String,
    pub description: String,
    pub suspended: bool,
    pub created_at: String,
    pub modified_at: String,
}

pub struct SystemOption {
    pub name: String,
    pub selected: bool,
}

impl From<Alias> for AliasView {
    fn from(a: Alias) -> Self {
        Self {
            full_alias: a.full_alias(),
            email_addresses: a.email_addresses.join(", "),
            description: a.description,
            suspended: a.suspended,
            created_at: a.created_at.format("%Y-%m-%d %H:%M").to_string(),
            modified_at: a.modified_at.format("%Y-%m-%d %H:%M").to_string(),
        }
    }
}

// -- Query Params --

#[derive(Debug, Deserialize, Default)]
pub struct AliasQuery {
    #[serde(default)]
    pub q: String,
    #[serde(default)]
    pub hide_suspended: bool,
    #[serde(default)]
    pub hide_enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct SystemForm {
    pub system: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateAliasForm {
    pub alias: String,
    pub domain: String,
    pub email_addresses: String,
    #[serde(default)]
    pub description: String,
}

// -- Templates --

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    aliases: Vec<AliasView>,
    systems: Vec<SystemOption>,
    query: String,
    hide_suspended: bool,
    hide_enabled: bool,
    alias_count: usize,
}

#[derive(Template)]
#[template(path = "partials/alias_rows.html")]
struct AliasRowsTemplate {
    aliases: Vec<AliasView>,
    alias_count: usize,
}

#[derive(Template)]
#[template(path = "partials/main_content.html")]
struct MainContentTemplate {
    aliases: Vec<AliasView>,
    query: String,
    hide_suspended: bool,
    hide_enabled: bool,
    alias_count: usize,
}

#[derive(Template)]
#[template(path = "partials/create_form.html")]
struct CreateFormTemplate {
    default_domain: String,
    default_addresses: String,
}

#[derive(Template)]
#[template(path = "partials/create_result.html")]
struct CreateResultTemplate {
    success: bool,
    message: String,
}

// -- Router --

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/", get(index_handler))
        .route("/aliases", get(aliases_handler))
        .route("/aliases/create", get(create_form_handler).post(create_alias_handler))
        .route("/system", post(system_handler))
        .route("/refresh", post(refresh_handler))
        .route("/static/{*path}", get(static_handler))
        .route("/health", get(health_handler))
        .with_state(state)
}

// -- Handlers --

async fn index_handler(State(state): State<SharedState>) -> Result<Html<String>, AppError> {
    let filter = AliasFilter::default();
    let aliases: Vec<AliasView> = state
        .list_aliases(&filter)
        .await?
        .into_iter()
        .map(AliasView::from)
        .collect();
    let alias_count = aliases.len();

    let active = state.active_system_name().await;
    let template = IndexTemplate {
        aliases,
        systems: build_system_options(&state, &active),
        query: String::new(),
        hide_suspended: false,
        hide_enabled: false,
        alias_count,
    };

    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("template render error: {}", e))
    })?))
}

async fn aliases_handler(
    State(state): State<SharedState>,
    Query(query): Query<AliasQuery>,
) -> Result<Html<String>, AppError> {
    let filter = query_to_filter(&query);
    let aliases: Vec<AliasView> = state
        .list_aliases(&filter)
        .await?
        .into_iter()
        .map(AliasView::from)
        .collect();
    let alias_count = aliases.len();

    let template = AliasRowsTemplate {
        aliases,
        alias_count,
    };

    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("template render error: {}", e))
    })?))
}

async fn system_handler(
    State(state): State<SharedState>,
    Form(form): Form<SystemForm>,
) -> Result<Html<String>, AppError> {
    state.switch_system(&form.system).await?;

    let filter = AliasFilter::default();
    let aliases: Vec<AliasView> = state
        .list_aliases(&filter)
        .await?
        .into_iter()
        .map(AliasView::from)
        .collect();
    let alias_count = aliases.len();

    let template = MainContentTemplate {
        aliases,
        query: String::new(),
        hide_suspended: false,
        hide_enabled: false,
        alias_count,
    };

    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("template render error: {}", e))
    })?))
}

async fn refresh_handler(
    State(state): State<SharedState>,
    Query(query): Query<AliasQuery>,
) -> Result<Html<String>, AppError> {
    state.refresh_active_system().await?;

    let filter = query_to_filter(&query);
    let aliases: Vec<AliasView> = state
        .list_aliases(&filter)
        .await?
        .into_iter()
        .map(AliasView::from)
        .collect();
    let alias_count = aliases.len();

    let template = AliasRowsTemplate {
        aliases,
        alias_count,
    };

    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("template render error: {}", e))
    })?))
}

async fn create_form_handler(
    State(state): State<SharedState>,
) -> Result<Html<String>, AppError> {
    let default_domain = state.active_default_domain().await.unwrap_or_default();
    let default_addresses = state
        .active_default_addresses()
        .await
        .map(|a| a.join(", "))
        .unwrap_or_default();

    let template = CreateFormTemplate {
        default_domain,
        default_addresses,
    };

    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("template render error: {}", e))
    })?))
}

async fn create_alias_handler(
    State(state): State<SharedState>,
    Form(form): Form<CreateAliasForm>,
) -> Result<impl IntoResponse, AppError> {
    let addresses: Vec<String> = form
        .email_addresses
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let alias = build_alias(
        form.alias.trim().to_string(),
        form.domain.trim().to_string(),
        addresses,
        form.description.trim().to_string(),
    );

    let (success, message) = match state.create_alias(alias).await {
        Ok(created) => (true, format!("Created alias {}", created.full_alias())),
        Err(e) => (false, format!("{}", e)),
    };

    let template = CreateResultTemplate { success, message };
    let html = template.render().map_err(|e| {
        AppError::Internal(format!("template render error: {}", e))
    })?;

    let mut response = Html(html).into_response();
    if success {
        response.headers_mut().insert(
            "HX-Trigger",
            "alias-created".parse().unwrap(),
        );
    }
    Ok(response)
}

async fn static_handler(Path(path): Path<String>) -> impl IntoResponse {
    match StaticAssets::get(&path) {
        Some(content) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, mime.as_ref().to_string()),
                    (
                        header::CACHE_CONTROL,
                        "public, max-age=31536000".to_string(),
                    ),
                ],
                content.data.into_owned(),
            )
                .into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn health_handler() -> &'static str {
    "ok"
}

// -- Helpers --

fn build_system_options(state: &SharedState, active: &str) -> Vec<SystemOption> {
    state
        .system_names()
        .into_iter()
        .map(|name| {
            let selected = name == active;
            SystemOption { name, selected }
        })
        .collect()
}

fn query_to_filter(query: &AliasQuery) -> AliasFilter {
    let regex = if query.q.trim().is_empty() {
        None
    } else {
        regex::Regex::new(&format!("(?i){}", regex::escape(query.q.trim()))).ok()
    };

    AliasFilter {
        alias: regex.clone(),
        domain: regex.clone(),
        description: regex.clone(),
        email_address: regex,
        exclude_suspended: query.hide_suspended,
        exclude_enabled: query.hide_enabled,
    }
}
