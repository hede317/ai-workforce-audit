use crate::auth::{audit, create_jwt, verify_password, AdminUser, AuthUser};
use crate::auth::new_id;
use crate::error::{ok, AppError, ApiResult};
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

pub async fn login(State(state): State<AppState>, Json(body): Json<Value>) -> ApiResult {
    let username = body.get("username").and_then(|v| v.as_str()).unwrap_or_default();
    let password = body.get("password").and_then(|v| v.as_str()).unwrap_or_default();
    let row: Option<(String, String, String, String)> =
        sqlx::query_as("SELECT id, username, password_hash, role FROM users WHERE username=$1")
            .bind(username)
            .fetch_optional(state.pool())
            .await?;
    let (id, uname, hash, role) = row.ok_or_else(|| AppError::unauthorized("用户名或密码错误"))?;
    if !verify_password(password, &hash) {
        return Err(AppError::unauthorized("用户名或密码错误"));
    }
    let token = create_jwt(&state.0.cfg.jwt_secret, &id, &uname, &role, &crate::auth::hash_signature(&hash))?;
    audit(state.pool(), &id, "login", "user", &id, json!({})).await;
    ok(json!({"token": token, "username": uname, "role": role}))
}

pub async fn me(State(state): State<AppState>, user: AuthUser) -> ApiResult {
    let _ = &state;
    ok(json!({"user_id": user.0.sub, "username": user.0.username, "role": user.0.role}))
}

pub async fn list_bus(State(state): State<AppState>, _user: AuthUser) -> ApiResult {
    let rows: Vec<(String, String, String, String, Option<String>, String, Option<Value>, String)> =
        sqlx::query_as("SELECT id, code, name, website, description, default_language, configuration, status FROM business_units ORDER BY code")
            .fetch_all(state.pool())
            .await?;
    let items: Vec<Value> = rows
        .into_iter()
        .map(|(id, code, name, website, description, lang, configuration, status)| {
            json!({"id": id, "code": code, "name": name, "website": website, "description": description, "default_language": lang, "configuration": configuration, "status": status})
        })
        .collect();
    ok(json!({"items": items, "total": items.len()}))
}

pub async fn create_bu(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(body): Json<Value>,
) -> ApiResult {
    let code = body.get("code").and_then(|v| v.as_str()).ok_or_else(|| AppError::bad_request("缺少 code"))?;
    let name = body.get("name").and_then(|v| v.as_str()).ok_or_else(|| AppError::bad_request("缺少 name"))?;
    let id = new_id();
    sqlx::query("INSERT INTO business_units (id, code, name, website, description, default_language, configuration) VALUES ($1,$2,$3,$4,$5,$6,$7)")
        .bind(&id)
        .bind(code)
        .bind(name)
        .bind(body.get("website").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(body.get("description").and_then(|v| v.as_str()))
        .bind(body.get("default_language").and_then(|v| v.as_str()).unwrap_or("zh"))
        .bind(body.get("configuration").cloned())
        .execute(state.pool())
        .await?;
    audit(state.pool(), &admin.0.sub, "create", "business_unit", &id, json!({"code": code})).await;
    ok(json!({"id": id, "code": code}))
}

pub async fn list_audit(State(state): State<AppState>, _admin: AdminUser, axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>) -> ApiResult {
    // 二审P2-9：审计日志分页（长期运行必备）
    let page: i64 = q.get("page").and_then(|v| v.parse().ok()).unwrap_or(1).clamp(1, 10000);
    let page_size: i64 = q.get("page_size").and_then(|v| v.parse().ok()).unwrap_or(100).clamp(1, 200);
    let rows: Vec<(String, Option<String>, String, Option<String>, Option<String>, Option<Value>, String)> =
        sqlx::query_as("SELECT id, user_id, action, entity, entity_id, detail, created_at::text FROM audit_logs ORDER BY created_at DESC LIMIT $1 OFFSET $2")
            .bind(page_size)
            .bind((page - 1) * page_size)
            .fetch_all(state.pool())
            .await?;
    let items: Vec<Value> = rows
        .into_iter()
        .map(|(id, user_id, action, entity, entity_id, detail, at)| {
            json!({"id": id, "user_id": user_id, "action": action, "entity": entity, "entity_id": entity_id, "detail": detail, "created_at": at})
        })
        .collect();
    ok(json!({"items": items}))
}
