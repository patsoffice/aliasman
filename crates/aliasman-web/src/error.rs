use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

pub enum AppError {
    Core(aliasman_core::error::Error),
    Internal(String),
}

impl From<aliasman_core::error::Error> for AppError {
    fn from(e: aliasman_core::error::Error) -> Self {
        AppError::Core(e)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::Core(e) => {
                tracing::error!("Core error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e))
            }
            AppError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Internal error: {}", msg),
                )
            }
        };

        let html = format!(
            r#"<div class="p-4 bg-red-50 border border-red-200 rounded text-red-700">{}</div>"#,
            message
        );
        (status, Html(html)).into_response()
    }
}
