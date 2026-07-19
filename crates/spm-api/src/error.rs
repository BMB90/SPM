use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use spm_storage::StorageError;

pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}

impl From<StorageError> for ApiError {
    fn from(e: StorageError) -> Self {
        match e {
            StorageError::NotFound => ApiError { status: StatusCode::NOT_FOUND, message: "not found".to_string() },
            other => ApiError { status: StatusCode::INTERNAL_SERVER_ERROR, message: other.to_string() },
        }
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        ApiError { status: StatusCode::INTERNAL_SERVER_ERROR, message: e.to_string() }
    }
}

pub fn bad_request(message: impl Into<String>) -> ApiError {
    ApiError { status: StatusCode::BAD_REQUEST, message: message.into() }
}
