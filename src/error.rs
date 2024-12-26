use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use redis::RedisError;

#[derive(Debug)]
pub enum AppError {
    RedisError(RedisError),
    JsonError(serde_json::Error),
    OrderNotFound(String),
    InvalidInput(String),
}

impl From<RedisError> for AppError {
    fn from(err: RedisError) -> Self {
        AppError::RedisError(err)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::JsonError(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::RedisError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            AppError::JsonError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            AppError::OrderNotFound(id) => (
                StatusCode::NOT_FOUND,
                format!("Order with id {} not found", id),
            ),
            AppError::InvalidInput(msg) => (StatusCode::BAD_REQUEST, msg),
        };

        (status, message).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
