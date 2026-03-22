use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Redirect};
use axum::routing::{get, post};
use axum::{Form, Router};
use axum_extra::extract::CookieJar;
use rust_embed::Embed;
use serde::Deserialize;

use aliasman_core::auth::Action;
use aliasman_core::build_alias;
use aliasman_core::model::{Alias, AliasFilter};

use crate::auth::{self, LoginForm, OptionalAuth, RequireAuth};
use crate::error::AppError;
use crate::state::SharedState;

// -- Static Assets --

#[derive(Embed)]
#[folder = "static/"]
struct StaticAssets;

// -- View Models --

pub struct AliasView {
    pub alias: String,
    pub domain: String,
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
        let full_alias = a.full_alias();
        Self {
            alias: a.alias,
            domain: a.domain,
            full_alias,
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
pub struct AliasActionForm {
    pub alias: String,
    pub domain: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateAliasForm {
    pub alias: String,
    pub domain: String,
    pub email_addresses: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct EditAliasForm {
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
    suspended_count: usize,
    auth_enabled: bool,
    username: String,
}

#[derive(Template)]
#[template(path = "partials/alias_rows.html")]
struct AliasRowsTemplate {
    aliases: Vec<AliasView>,
    alias_count: usize,
    suspended_count: usize,
}

#[derive(Template)]
#[template(path = "partials/main_content.html")]
struct MainContentTemplate {
    aliases: Vec<AliasView>,
    query: String,
    hide_suspended: bool,
    hide_enabled: bool,
    alias_count: usize,
    suspended_count: usize,
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

#[derive(Template)]
#[template(path = "partials/action_result.html")]
struct ActionResultTemplate {
    success: bool,
    message: String,
}

#[derive(Template)]
#[template(path = "partials/edit_form.html")]
struct EditFormTemplate {
    alias: String,
    domain: String,
    email_addresses: String,
    description: String,
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    error_message: String,
}

// -- Router --

pub fn router(state: SharedState) -> Router {
    Router::new()
        // Public routes
        .route("/login", get(login_page_handler).post(login_submit_handler))
        .route("/logout", post(logout_handler))
        .route("/static/{*path}", get(static_handler))
        .route("/health", get(health_handler))
        // Protected routes (RequireAuth extractor handles redirect)
        .route("/", get(index_handler))
        .route("/aliases", get(aliases_handler))
        .route(
            "/aliases/create",
            get(create_form_handler).post(create_alias_handler),
        )
        .route(
            "/aliases/edit",
            get(edit_form_handler).post(edit_alias_handler),
        )
        .route("/aliases/delete", post(delete_alias_handler))
        .route("/aliases/suspend", post(suspend_alias_handler))
        .route("/aliases/unsuspend", post(unsuspend_alias_handler))
        .route("/system", post(system_handler))
        .route("/refresh", get(refresh_handler))
        .with_state(state)
}

// -- Auth Handlers --

async fn login_page_handler(
    State(state): State<SharedState>,
    auth: OptionalAuth,
) -> impl IntoResponse {
    // If already authenticated, redirect to home
    if auth.0.is_some() {
        return Redirect::to("/").into_response();
    }

    // If auth is not configured, redirect to home (no login needed)
    if !state.auth_enabled() {
        return Redirect::to("/").into_response();
    }

    let template = LoginTemplate {
        error_message: String::new(),
    };
    Html(
        template
            .render()
            .unwrap_or_else(|e| format!("template error: {}", e)),
    )
    .into_response()
}

async fn login_submit_handler(
    State(state): State<SharedState>,
    jar: CookieJar,
    Form(form): Form<LoginForm>,
) -> impl IntoResponse {
    match auth::login_handler(state, jar, form).await {
        Ok((jar, redirect)) => (jar, redirect).into_response(),
        Err((_status, message)) => {
            let template = LoginTemplate {
                error_message: message,
            };
            Html(
                template
                    .render()
                    .unwrap_or_else(|e| format!("template error: {}", e)),
            )
            .into_response()
        }
    }
}

async fn logout_handler(State(state): State<SharedState>, jar: CookieJar) -> impl IntoResponse {
    auth::logout_handler(state, jar).await
}

// -- Protected Handlers --

async fn index_handler(
    State(state): State<SharedState>,
    auth: RequireAuth,
) -> Result<Html<String>, AppError> {
    let filter = AliasFilter::default();
    let aliases: Vec<AliasView> = state
        .list_aliases(&filter)
        .await?
        .into_iter()
        .map(AliasView::from)
        .collect();
    let alias_count = aliases.len();
    let suspended_count = aliases.iter().filter(|a| a.suspended).count();

    let active = state.active_system_name().await;
    let template = IndexTemplate {
        aliases,
        systems: build_system_options(&state, &active),
        query: String::new(),
        hide_suspended: false,
        hide_enabled: false,
        alias_count,
        suspended_count,
        auth_enabled: state.auth_enabled(),
        username: auth.session().username.clone(),
    };

    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("template render error: {}", e))
    })?))
}

async fn aliases_handler(
    State(state): State<SharedState>,
    _auth: RequireAuth,
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
    let suspended_count = aliases.iter().filter(|a| a.suspended).count();

    let template = AliasRowsTemplate {
        aliases,
        alias_count,
        suspended_count,
    };

    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("template render error: {}", e))
    })?))
}

async fn system_handler(
    State(state): State<SharedState>,
    _auth: RequireAuth,
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
    let suspended_count = aliases.iter().filter(|a| a.suspended).count();

    let template = MainContentTemplate {
        aliases,
        query: String::new(),
        hide_suspended: false,
        hide_enabled: false,
        alias_count,
        suspended_count,
    };

    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("template render error: {}", e))
    })?))
}

async fn refresh_handler(
    State(state): State<SharedState>,
    _auth: RequireAuth,
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
    let suspended_count = aliases.iter().filter(|a| a.suspended).count();

    let template = AliasRowsTemplate {
        aliases,
        alias_count,
        suspended_count,
    };

    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("template render error: {}", e))
    })?))
}

async fn create_form_handler(
    State(state): State<SharedState>,
    _auth: RequireAuth,
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
    auth: RequireAuth,
    Form(form): Form<CreateAliasForm>,
) -> Result<impl IntoResponse, AppError> {
    let domain = form.domain.trim();
    if !state
        .check_permission(auth.session(), &Action::Create, domain)
        .await
    {
        return Err(AppError::Unauthorized(format!(
            "no create permission on domain '{}'",
            domain
        )));
    }

    let addresses: Vec<String> = form
        .email_addresses
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let alias = build_alias(
        form.alias.trim().to_string(),
        domain.to_string(),
        addresses,
        form.description.trim().to_string(),
    );

    let (success, message) = match state.create_alias(alias).await {
        Ok(created) => (true, format!("Created alias {}", created.full_alias())),
        Err(e) => (false, format!("{}", e)),
    };

    let template = CreateResultTemplate { success, message };
    let html = template
        .render()
        .map_err(|e| AppError::Internal(format!("template render error: {}", e)))?;

    let mut response = Html(html).into_response();
    if success {
        response
            .headers_mut()
            .insert("HX-Trigger", "alias-changed".parse().unwrap());
    }
    Ok(response)
}

async fn edit_form_handler(
    State(state): State<SharedState>,
    auth: RequireAuth,
    Query(form): Query<AliasActionForm>,
) -> Result<Html<String>, AppError> {
    if !state
        .check_permission(auth.session(), &Action::Create, &form.domain)
        .await
    {
        return Err(AppError::Unauthorized(format!(
            "no edit permission on domain '{}'",
            form.domain
        )));
    }
    let filter = AliasFilter::default();
    let aliases = state.list_aliases(&filter).await?;
    let existing = aliases
        .into_iter()
        .find(|a| a.alias == form.alias && a.domain == form.domain)
        .ok_or_else(|| {
            AppError::Internal(format!("alias '{}@{}' not found", form.alias, form.domain))
        })?;

    let template = EditFormTemplate {
        alias: existing.alias,
        domain: existing.domain,
        email_addresses: existing.email_addresses.join(", "),
        description: existing.description,
    };

    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("template render error: {}", e))
    })?))
}

async fn edit_alias_handler(
    State(state): State<SharedState>,
    auth: RequireAuth,
    Form(form): Form<EditAliasForm>,
) -> Result<impl IntoResponse, AppError> {
    if !state
        .check_permission(auth.session(), &Action::Create, &form.domain)
        .await
    {
        return Err(AppError::Unauthorized(format!(
            "no edit permission on domain '{}'",
            form.domain
        )));
    }

    let addresses: Vec<String> = form
        .email_addresses
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let result = state
        .edit_alias(
            &form.alias,
            &form.domain,
            Some(addresses),
            Some(form.description.trim().to_string()),
        )
        .await;

    alias_action_response(
        result.map(|_| ()),
        format!("Updated alias {}@{}", form.alias, form.domain),
    )
}

async fn delete_alias_handler(
    State(state): State<SharedState>,
    auth: RequireAuth,
    Form(form): Form<AliasActionForm>,
) -> Result<impl IntoResponse, AppError> {
    if !state
        .check_permission(auth.session(), &Action::Delete, &form.domain)
        .await
    {
        return Err(AppError::Unauthorized(format!(
            "no delete permission on domain '{}'",
            form.domain
        )));
    }

    alias_action_response(
        state.delete_alias(&form.alias, &form.domain).await,
        format!("Deleted alias {}@{}", form.alias, form.domain),
    )
}

async fn suspend_alias_handler(
    State(state): State<SharedState>,
    auth: RequireAuth,
    Form(form): Form<AliasActionForm>,
) -> Result<impl IntoResponse, AppError> {
    if !state
        .check_permission(auth.session(), &Action::Suspend, &form.domain)
        .await
    {
        return Err(AppError::Unauthorized(format!(
            "no suspend permission on domain '{}'",
            form.domain
        )));
    }

    alias_action_response(
        state.suspend_alias(&form.alias, &form.domain).await,
        format!("Suspended alias {}@{}", form.alias, form.domain),
    )
}

async fn unsuspend_alias_handler(
    State(state): State<SharedState>,
    auth: RequireAuth,
    Form(form): Form<AliasActionForm>,
) -> Result<impl IntoResponse, AppError> {
    if !state
        .check_permission(auth.session(), &Action::Unsuspend, &form.domain)
        .await
    {
        return Err(AppError::Unauthorized(format!(
            "no unsuspend permission on domain '{}'",
            form.domain
        )));
    }

    alias_action_response(
        state.unsuspend_alias(&form.alias, &form.domain).await,
        format!("Unsuspended alias {}@{}", form.alias, form.domain),
    )
}

fn alias_action_response(
    result: aliasman_core::error::Result<()>,
    success_message: String,
) -> Result<axum::response::Response, AppError> {
    let (success, message) = match result {
        Ok(()) => (true, success_message),
        Err(e) => (false, format!("{}", e)),
    };

    let template = ActionResultTemplate { success, message };
    let html = template
        .render()
        .map_err(|e| AppError::Internal(format!("template render error: {}", e)))?;

    let mut response = Html(html).into_response();
    if success {
        response
            .headers_mut()
            .insert("HX-Trigger", "alias-changed".parse().unwrap());
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
