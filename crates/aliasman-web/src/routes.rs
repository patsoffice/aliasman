use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Redirect};
use axum::routing::{get, post};
use axum::{Form, Router};
use axum_extra::extract::CookieJar;
use rust_embed::Embed;
use serde::Deserialize;

use aliasman_core::auth::{Action, NewUser, Permission, ResourceType, Session};
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

// -- Admin View Models --

pub struct UserView {
    pub id: String,
    pub username: String,
    pub is_superuser: bool,
    pub permissions: Vec<PermissionView>,
}

pub struct PermissionView {
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserForm {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub is_superuser: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GrantPermissionForm {
    pub resource_type: String,
    pub resource_id: String,
    #[serde(default)]
    pub actions: String,
}

#[derive(Debug, Deserialize)]
pub struct RevokePermissionForm {
    pub resource_type: String,
    pub resource_id: String,
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
    is_superuser: bool,
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

#[derive(Template)]
#[template(path = "admin.html")]
struct AdminTemplate {
    users: Vec<UserView>,
    username: String,
}

#[derive(Template)]
#[template(path = "partials/admin_user_rows.html")]
struct AdminUserRowsTemplate {
    users: Vec<UserView>,
}

#[derive(Template)]
#[template(path = "partials/admin_create_user.html")]
struct AdminCreateUserTemplate;

#[derive(Template)]
#[template(path = "partials/admin_permissions.html")]
struct AdminPermissionsTemplate {
    user_id: String,
    username: String,
    permissions: Vec<PermissionView>,
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
        // Admin routes (superuser only)
        .route("/admin/users", get(admin_users_handler))
        .route("/admin/users/rows", get(admin_users_rows_handler))
        .route("/admin/users/create", get(admin_create_user_form_handler))
        .route("/admin/users", post(admin_create_user_handler))
        .route(
            "/admin/users/{user_id}/delete",
            post(admin_delete_user_handler),
        )
        .route(
            "/admin/users/{user_id}/permissions",
            get(admin_permissions_handler),
        )
        .route(
            "/admin/users/{user_id}/permissions/grant",
            post(admin_grant_handler),
        )
        .route(
            "/admin/users/{user_id}/permissions/revoke",
            post(admin_revoke_handler),
        )
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
        is_superuser: auth.session().is_superuser,
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

// -- Admin Handlers --

fn require_superuser(session: &Session) -> Result<(), AppError> {
    if session.is_superuser {
        Ok(())
    } else {
        Err(AppError::Unauthorized(
            "superuser access required".to_string(),
        ))
    }
}

async fn load_user_views(state: &SharedState) -> Result<Vec<UserView>, AppError> {
    let store = state
        .user_store()
        .ok_or_else(|| AppError::Internal("auth not configured".to_string()))?;

    let users = store
        .list_users()
        .await
        .map_err(|e| AppError::Internal(format!("failed to list users: {}", e)))?;

    let mut views = Vec::new();
    for user in users {
        let perms = store
            .get_permissions(&user.id)
            .await
            .map_err(|e| AppError::Internal(format!("failed to get permissions: {}", e)))?;

        let perm_views: Vec<PermissionView> = perms
            .iter()
            .map(|p| PermissionView {
                action: p.action.as_str().to_string(),
                resource_type: p.resource_type.as_str().to_string(),
                resource_id: p.resource_id.clone().unwrap_or_else(|| "*".to_string()),
            })
            .collect();

        views.push(UserView {
            id: user.id,
            username: user.username,
            is_superuser: user.is_superuser,
            permissions: perm_views,
        });
    }
    Ok(views)
}

async fn admin_users_handler(
    State(state): State<SharedState>,
    auth: RequireAuth,
) -> Result<Html<String>, AppError> {
    require_superuser(auth.session())?;
    let users = load_user_views(&state).await?;

    let template = AdminTemplate {
        users,
        username: auth.session().username.clone(),
    };
    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("template render error: {}", e))
    })?))
}

async fn admin_users_rows_handler(
    State(state): State<SharedState>,
    auth: RequireAuth,
) -> Result<Html<String>, AppError> {
    require_superuser(auth.session())?;
    let users = load_user_views(&state).await?;

    let template = AdminUserRowsTemplate { users };
    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("template render error: {}", e))
    })?))
}

async fn admin_create_user_form_handler(auth: RequireAuth) -> Result<Html<String>, AppError> {
    require_superuser(auth.session())?;
    let template = AdminCreateUserTemplate;
    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("template render error: {}", e))
    })?))
}

async fn admin_create_user_handler(
    State(state): State<SharedState>,
    auth: RequireAuth,
    Form(form): Form<CreateUserForm>,
) -> Result<impl IntoResponse, AppError> {
    require_superuser(auth.session())?;

    let store = state
        .user_store()
        .ok_or_else(|| AppError::Internal("auth not configured".to_string()))?;

    let new_user = NewUser {
        username: form.username.trim().to_string(),
        password: form.password,
        is_superuser: form.is_superuser.as_deref() == Some("true"),
    };

    let (success, message) = match store.create_user(&new_user).await {
        Ok(user) => (true, format!("Created user '{}'", user.username)),
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
            .insert("HX-Trigger", "user-changed".parse().unwrap());
    }
    Ok(response)
}

async fn admin_delete_user_handler(
    State(state): State<SharedState>,
    auth: RequireAuth,
    Path(user_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    require_superuser(auth.session())?;

    let store = state
        .user_store()
        .ok_or_else(|| AppError::Internal("auth not configured".to_string()))?;

    // Look up the user to get username for the message and prevent self-deletion
    let user = store
        .get_user(&user_id)
        .await
        .map_err(|e| AppError::Internal(format!("{}", e)))?
        .ok_or_else(|| AppError::Internal("user not found".to_string()))?;

    if user.id == auth.session().user_id {
        return alias_action_response(
            Err(aliasman_core::error::Error::InvalidInput(
                "cannot delete yourself".to_string(),
            )),
            String::new(),
        );
    }

    let (success, message) = match store.delete_user(&user.username).await {
        Ok(()) => (true, format!("Deleted user '{}'", user.username)),
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
            .insert("HX-Trigger", "user-changed".parse().unwrap());
    }
    Ok(response)
}

async fn admin_permissions_handler(
    State(state): State<SharedState>,
    auth: RequireAuth,
    Path(user_id): Path<String>,
) -> Result<Html<String>, AppError> {
    require_superuser(auth.session())?;

    let store = state
        .user_store()
        .ok_or_else(|| AppError::Internal("auth not configured".to_string()))?;

    let user = store
        .get_user(&user_id)
        .await
        .map_err(|e| AppError::Internal(format!("{}", e)))?
        .ok_or_else(|| AppError::Internal("user not found".to_string()))?;

    let perms = store
        .get_permissions(&user_id)
        .await
        .map_err(|e| AppError::Internal(format!("{}", e)))?;

    let permissions: Vec<PermissionView> = perms
        .iter()
        .map(|p| PermissionView {
            action: p.action.as_str().to_string(),
            resource_type: p.resource_type.as_str().to_string(),
            resource_id: p.resource_id.clone().unwrap_or_else(|| "*".to_string()),
        })
        .collect();

    let template = AdminPermissionsTemplate {
        user_id,
        username: user.username,
        permissions,
    };
    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("template render error: {}", e))
    })?))
}

async fn admin_grant_handler(
    State(state): State<SharedState>,
    auth: RequireAuth,
    Path(user_id): Path<String>,
    Form(form): Form<GrantPermissionForm>,
) -> Result<Html<String>, AppError> {
    require_superuser(auth.session())?;

    let store = state
        .user_store()
        .ok_or_else(|| AppError::Internal("auth not configured".to_string()))?;

    let resource_type = ResourceType::parse(&form.resource_type)
        .map_err(|e| AppError::Internal(format!("{}", e)))?;

    let actions: Vec<Action> = if form.actions.trim().is_empty() {
        Action::all().to_vec()
    } else {
        form.actions
            .split(',')
            .map(|s| Action::parse(s.trim()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Internal(format!("{}", e)))?
    };

    let permissions: Vec<Permission> = actions
        .iter()
        .map(|action| Permission {
            id: String::new(),
            user_id: user_id.clone(),
            action: action.clone(),
            resource_type: resource_type.clone(),
            resource_id: Some(form.resource_id.trim().to_string()),
        })
        .collect();

    store
        .set_permissions(&user_id, &permissions)
        .await
        .map_err(|e| AppError::Internal(format!("{}", e)))?;

    // Re-render the permissions form
    admin_permissions_handler(State(state), auth, Path(user_id)).await
}

async fn admin_revoke_handler(
    State(state): State<SharedState>,
    auth: RequireAuth,
    Path(user_id): Path<String>,
    Form(form): Form<RevokePermissionForm>,
) -> Result<Html<String>, AppError> {
    require_superuser(auth.session())?;

    let store = state
        .user_store()
        .ok_or_else(|| AppError::Internal("auth not configured".to_string()))?;

    let resource_type = ResourceType::parse(&form.resource_type)
        .map_err(|e| AppError::Internal(format!("{}", e)))?;

    store
        .clear_permissions(&user_id, &resource_type, &form.resource_id)
        .await
        .map_err(|e| AppError::Internal(format!("{}", e)))?;

    // Re-render the permissions form
    admin_permissions_handler(State(state), auth, Path(user_id)).await
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
