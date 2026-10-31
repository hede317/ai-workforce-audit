use crate::error::AppError;
use crate::state::AppState;
use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, SaltString};
use argon2::{Argon2, PasswordVerifier};
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::internal(format!("密码哈希失败: {e}")))
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .ok()
        .map(|parsed| Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
        .unwrap_or(false)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,       // user id
    pub username: String,
    pub role: String,
    pub exp: i64,
}

pub fn create_jwt(secret: &str, user_id: &str, username: &str, role: &str) -> Result<String, AppError> {
    let claims = Claims {
        sub: user_id.into(),
        username: username.into(),
        role: role.into(),
        exp: (Utc::now() + Duration::hours(24)).timestamp(),
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
        .map_err(|e| AppError::internal(format!("签发令牌失败: {e}")))
}

pub fn decode_jwt(secret: &str, token: &str) -> Result<Claims, AppError> {
    decode::<Claims>(token, &DecodingKey::from_secret(secret.as_bytes()), &Validation::default())
        .map(|d| d.claims)
        .map_err(|_| AppError::unauthorized("令牌无效或已过期"))
}

/// 需要登录的提取器：`AuthUser(user)`
pub struct AuthUser(pub Claims);

#[axum::async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::unauthorized("缺少 Authorization 头"))?;
        let token = header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AppError::unauthorized("Authorization 必须为 Bearer 令牌"))?;
        let claims = decode_jwt(&state.0.cfg.jwt_secret, token)?;
        Ok(AuthUser(claims))
    }
}

/// 需要管理员角色的提取器
pub struct AdminUser(pub Claims);

#[axum::async_trait]
impl FromRequestParts<AppState> for AdminUser {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if user.0.role != "ADMIN" {
            return Err(AppError::forbidden("需要 ADMIN 角色"));
        }
        Ok(AdminUser(user.0))
    }
}

pub fn new_id() -> String {
    Uuid::new_v4().simple().to_string()
}

/// 写审计日志（尽力而为，不阻塞业务）
pub async fn audit(pool: &sqlx::PgPool, user_id: &str, action: &str, entity: &str, entity_id: &str, detail: serde_json::Value) {
    let _ = sqlx::query(
        "INSERT INTO audit_logs (id, user_id, action, entity, entity_id, detail) VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(new_id())
    .bind(user_id)
    .bind(action)
    .bind(entity)
    .bind(entity_id)
    .bind(serde_json::to_string(&detail).unwrap_or_default())
    .execute(pool)
    .await;
}

/// 审批拍板权（红线：仅 ADMIN）
pub struct ApproverUser(pub Claims);

#[axum::async_trait]
impl FromRequestParts<AppState> for ApproverUser {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if user.0.role != "ADMIN" {
            return Err(AppError::forbidden("审批拍板需要 ADMIN 角色（不能自己批自己的任务）"));
        }
        Ok(ApproverUser(user.0))
    }
}

/// 内容与知识编辑权（ADMIN | EDITOR）
pub struct EditorUser(pub Claims);

#[axum::async_trait]
impl FromRequestParts<AppState> for EditorUser {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if user.0.role != "ADMIN" && user.0.role != "EDITOR" {
            return Err(AppError::forbidden("需要 EDITOR 或 ADMIN 角色"));
        }
        Ok(EditorUser(user.0))
    }
}
