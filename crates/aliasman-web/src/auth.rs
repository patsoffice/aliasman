//! Authentication middleware and extractors for the web frontend.
//!
//! When auth is configured, requests are authenticated via a session cookie.
//! When auth is not configured, all requests get anonymous superuser access.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_extra::extract::CookieJar;

use aliasman_core::auth::Session;

use crate::state::SharedState;

/// Cookie name for the session token.
pub const SESSION_COOKIE: &str = "aliasman_session";

/// Extractor that provides the current session.
///
/// - If auth is configured: reads the session cookie, validates it against the
///   user store, and returns the session. Redirects to `/login` if invalid.
/// - If auth is not configured: returns an anonymous superuser session.
#[derive(Debug, Clone)]
pub struct RequireAuth(pub Session);

impl RequireAuth {
    pub fn session(&self) -> &Session {
        &self.0
    }
}

/// Error type for auth extraction failures — redirects to login page.
pub struct AuthRedirect;

impl IntoResponse for AuthRedirect {
    fn into_response(self) -> Response {
        Redirect::to("/login").into_response()
    }
}

impl FromRequestParts<SharedState> for RequireAuth {
    type Rejection = AuthRedirect;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &SharedState,
    ) -> Result<Self, Self::Rejection> {
        let user_store = state.user_store();

        // If auth is not configured, return anonymous superuser
        let Some(store) = user_store else {
            return Ok(RequireAuth(anonymous_session()));
        };

        // Extract cookies from the request
        let jar = CookieJar::from_headers(&parts.headers);

        let token = jar
            .get(SESSION_COOKIE)
            .map(|c| c.value().to_string())
            .ok_or(AuthRedirect)?;

        let session = store.get_session(&token).await.map_err(|_| AuthRedirect)?;

        Ok(RequireAuth(session))
    }
}

/// Optional auth extractor — returns None instead of redirecting when not authenticated.
/// Useful for routes that behave differently when logged in vs not (e.g., login page).
#[derive(Debug, Clone)]
pub struct OptionalAuth(pub Option<Session>);

impl FromRequestParts<SharedState> for OptionalAuth {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &SharedState,
    ) -> Result<Self, Self::Rejection> {
        let user_store = state.user_store();

        let Some(store) = user_store else {
            return Ok(OptionalAuth(Some(anonymous_session())));
        };

        let jar = CookieJar::from_headers(&parts.headers);

        let token = jar.get(SESSION_COOKIE).map(|c| c.value().to_string());

        if let Some(token) = token {
            match store.get_session(&token).await {
                Ok(s) => Ok(OptionalAuth(Some(s))),
                Err(_) => Ok(OptionalAuth(None)),
            }
        } else {
            Ok(OptionalAuth(None))
        }
    }
}

fn anonymous_session() -> Session {
    Session {
        token: String::new(),
        user_id: "anonymous".to_string(),
        username: "anonymous".to_string(),
        is_superuser: true,
    }
}

/// Login form data.
#[derive(serde::Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

/// Handle POST /login — authenticate and set session cookie.
pub async fn login_handler(
    state: SharedState,
    jar: CookieJar,
    form: LoginForm,
) -> Result<(CookieJar, Redirect), (StatusCode, String)> {
    let store = state.user_store().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "auth not configured".to_string(),
    ))?;

    let session = store
        .authenticate(&form.username, &form.password)
        .await
        .map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                "Invalid username or password".to_string(),
            )
        })?;

    let cookie = Cookie::build((SESSION_COOKIE, session.token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .build();

    Ok((jar.add(cookie), Redirect::to("/")))
}

/// Handle POST /logout — delete session and clear cookie.
pub async fn logout_handler(state: SharedState, jar: CookieJar) -> (CookieJar, Redirect) {
    if let Some(store) = state.user_store() {
        if let Some(cookie) = jar.get(SESSION_COOKIE) {
            let _ = store.delete_session(cookie.value()).await;
        }
    }

    let removal = Cookie::build(SESSION_COOKIE).path("/").build();

    (jar.remove(removal), Redirect::to("/login"))
}
