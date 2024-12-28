use async_openai::error::OpenAIError;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use redis::RedisError;
use std::io;
use std::sync::PoisonError;

/// Custom error types for the application
#[derive(Debug)]
pub enum AppError {
    /// Redis operation errors
    RedisError(RedisError),
    /// JSON serialization/deserialization errors
    JsonSerializationError(serde_json::Error),
    /// Plain text serialization errors
    PlainSerializationError(serde_plain::Error),
    /// Error when an order cannot be found
    OrderNotFound(String),
    /// Invalid input parameters
    InvalidInput(String),
    /// File I/O errors
    IoError(io::Error),
    /// Mutex lock acquisition errors
    LockError,
    /// OpenAI API errors
    OpenAIError(OpenAIError),
}

/// Type alias for Results that use AppError as the error type
pub type AppResult<T> = Result<T, AppError>;

impl From<RedisError> for AppError {
    /// Converts Redis errors into AppError
    fn from(err: RedisError) -> Self {
        AppError::RedisError(err)
    }
}

impl From<serde_json::Error> for AppError {
    /// Converts JSON serialization errors into AppError
    fn from(err: serde_json::Error) -> Self {
        AppError::JsonSerializationError(err)
    }
}

impl From<serde_plain::Error> for AppError {
    /// Converts plain text serialization errors into AppError
    fn from(err: serde_plain::Error) -> Self {
        AppError::PlainSerializationError(err)
    }
}

impl From<io::Error> for AppError {
    /// Converts I/O errors into AppError
    fn from(err: io::Error) -> Self {
        AppError::IoError(err)
    }
}

impl From<OpenAIError> for AppError {
    /// Converts OpenAI API errors into AppError
    fn from(err: OpenAIError) -> Self {
        AppError::OpenAIError(err)
    }
}

impl<T> From<PoisonError<T>> for AppError {
    /// Converts mutex poisoning errors into AppError
    fn from(_: PoisonError<T>) -> Self {
        AppError::LockError
    }
}

impl IntoResponse for AppError {
    /// Converts AppError into an HTTP response
    ///
    /// # Returns
    /// * `Response` - HTTP response with appropriate status code and error message
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
