use async_openai::error::OpenAIError;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use redis::RedisError;
use std::io;
use std::sync::PoisonError;

#[derive(Debug)]
pub enum AppError {
    RedisError(RedisError),
    JsonSerializationError(serde_json::Error),
    PlainSerializationError(serde_plain::Error),
    OrderNotFound(String),
    InvalidInput(String),
    IoError(io::Error),
    LockError,
    OpenAIError(OpenAIError),
}

impl From<RedisError> for AppError {
    fn from(err: RedisError) -> Self {
        AppError::RedisError(err)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::JsonSerializationError(err)
    }
}

impl From<serde_plain::Error> for AppError {
    fn from(err: serde_plain::Error) -> Self {
        AppError::PlainSerializationError(err)
    }
}

impl From<io::Error> for AppError {
    fn from(err: io::Error) -> Self {
        AppError::IoError(err)
    }
}

impl From<OpenAIError> for AppError {
    fn from(err: OpenAIError) -> Self {
        AppError::OpenAIError(err)
    }
}

impl<T> From<PoisonError<T>> for AppError {
    fn from(_: PoisonError<T>) -> Self {
        AppError::LockError
    }
}
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::RedisError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            AppError::JsonSerializationError(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
            AppError::PlainSerializationError(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
            AppError::OrderNotFound(id) => (
                StatusCode::NOT_FOUND,
                format!("Order with id {} not found", id),
            ),
            AppError::InvalidInput(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::IoError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            AppError::OpenAIError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            AppError::LockError => (StatusCode::INTERNAL_SERVER_ERROR, "Lock error".to_string()),
        };

        (status, message).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
