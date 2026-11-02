use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub type ApiResult<T> = std::result::Result<T, ApiError>;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Internal(String),
}

impl From<color_eyre::Report> for ApiError {
    fn from(e: color_eyre::Report) -> Self {
        ApiError::Internal(format!("{e:#}"))
    }
}
impl From<reqwest::Error> for ApiError {
    fn from(e: reqwest::Error) -> Self {
        ApiError::Internal(e.to_string())
    }
}
impl From<std::io::Error> for ApiError {
    fn from(e: std::io::Error) -> Self {
        ApiError::Internal(e.to_string())
    }
}
impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        ApiError::BadRequest(e.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self {
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let msg = self.to_string();
        (status, Json(serde_json::json!({ "error": msg }))).into_response()
    }
}
