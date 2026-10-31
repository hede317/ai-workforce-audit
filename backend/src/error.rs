use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};

/// 统一响应封套：{"success":bool,"data":...,"error":...,"request_id":...}
#[derive(Debug, thiserror::Error)]
#[error("{code}: {message}")]
pub struct AppError {
    pub status: StatusCode,
    pub code: String,
    pub message: String,
}

impl AppError {
    pub fn new(status: StatusCode, code: &str, message: impl Into<String>) -> Self {
        Self { status, code: code.into(), message: message.into() }
    }
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "BAD_REQUEST", msg)
    }
    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg)
    }
    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, "FORBIDDEN", msg)
    }
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "NOT_FOUND", msg)
    }
    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, "CONFLICT", msg)
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL", msg)
    }
    /// 外部能力未配置（AI/平台API/n8n），按规范要求返回真实待配置状态而非假数据
    pub fn not_configured(what: &str) -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "NOT_CONFIGURED",
            format!("{what} 未配置：请在环境变量中提供对应凭据后重试"),
        )
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = json!({
            "success": false,
            "data": Value::Null,
            "error": { "code": self.code, "message": self.message },
            "request_id": Value::Null,
        });
        (self.status, Json(body)).into_response()
    }
}

pub type ApiResult = Result<Response, AppError>;

pub fn ok(data: Value) -> ApiResult {
    let body = json!({
        "success": true,
        "data": data,
        "error": Value::Null,
        "request_id": Value::Null,
    });
    Ok((StatusCode::OK, Json(body)).into_response())
}

pub fn created(data: Value) -> ApiResult {
    let body = json!({
        "success": true,
        "data": data,
        "error": Value::Null,
        "request_id": Value::Null,
    });
    Ok((StatusCode::CREATED, Json(body)).into_response())
}

/// 数据库错误转换
impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        match &e {
            sqlx::Error::RowNotFound => AppError::not_found("记录不存在"),
            sqlx::Error::Database(db) if db.is_unique_violation() => {
                AppError::conflict("记录已存在（唯一约束冲突）")
            }
            _ => {
                tracing::error!("db error: {e}");
                AppError::internal("数据库错误")
            }
        }
    }
}
