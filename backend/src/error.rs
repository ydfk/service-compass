use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    Validation(String),
    #[error("未登录或会话已失效")]
    Unauthorized,
    #[error("资源不存在")]
    NotFound,
    #[error("资源仍被引用，无法删除")]
    Conflict,
    #[error("数据库操作失败")]
    Database(#[from] sqlx::Error),
    #[error("{0}")]
    External(String),
    #[error("内部服务错误")]
    Internal(#[from] anyhow::Error),
}

#[derive(Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, code) = match &self {
            Self::Validation(_) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR"),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
            Self::NotFound => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            Self::Conflict => (StatusCode::CONFLICT, "CONFLICT"),
            Self::Database(error) => {
                tracing::error!(?error, "数据库操作失败");
                (StatusCode::INTERNAL_SERVER_ERROR, "DATABASE_ERROR")
            }
            Self::External(_) => (StatusCode::BAD_GATEWAY, "EXTERNAL_ERROR"),
            Self::Internal(error) => {
                tracing::error!(?error, "内部服务错误");
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR")
            }
        };
        let body = ErrorBody {
            error: ErrorDetail {
                code,
                message: self.to_string(),
            },
        };
        (status, Json(body)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
