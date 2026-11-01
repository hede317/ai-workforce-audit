use crate::auth::{audit, new_id, AuthUser};
use crate::error::{ok, ApiResult, AppError};

fn truncate(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::Json;
use serde_json::{json, Value};
use std::collections::HashMap;

pub async fn list_keywords(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResult {
    let rows: Vec<(String, Option<String>, Option<String>, String, String, bool)> =
        sqlx::query_as("SELECT id, business_unit_code, goal_text, keyword, platform, enabled FROM listening_keywords WHERE ($1::text IS NULL OR business_unit_code=$1) ORDER BY created_at DESC LIMIT 200")
            .bind(q.get("business_unit_code"))
            .fetch_all(state.pool())
            .await?;
    let items: Vec<Value> = rows
        .into_iter()
        .map(|(id, bu, goal, kw, platform, enabled)| json!({"id": id, "business_unit_code": bu, "goal_text": goal, "keyword": kw, "platform": platform, "enabled": enabled}))
        .collect();
    ok(json!({"items": items, "total": items.len()}))
}

pub async fn add_keyword(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<Value>,
) -> ApiResult {
    let keyword = body
        .get("keyword")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::bad_request("缺少 keyword"))?;
    let id = new_id();
    sqlx::query("INSERT INTO listening_keywords (id, business_unit_code, goal_text, keyword, platform) VALUES ($1,$2,$3,$4,$5)")
        .bind(&id)
        .bind(body.get("business_unit_code").and_then(|v| v.as_str()))
        .bind(body.get("goal_text").and_then(|v| v.as_str()))
        .bind(keyword)
        .bind(body.get("platform").and_then(|v| v.as_str()).unwrap_or("news"))
        .execute(state.pool())
        .await?;
    audit(
        state.pool(),
        &user.0.sub,
        "create",
        "listening_keyword",
        &id,
        json!({"keyword": keyword}),
    )
    .await;
    ok(json!({"id": id}))
}

pub async fn delete_keyword(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> ApiResult {
    sqlx::query("DELETE FROM listening_keywords WHERE id=$1")
        .bind(&id)
        .execute(state.pool())
        .await?;
    audit(
        state.pool(),
        &user.0.sub,
        "delete",
        "listening_keyword",
        &id,
        json!({}),
    )
    .await;
    ok(json!({"deleted": true}))
}

pub async fn list_signals(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResult {
    let rows: Vec<(String, Option<String>, Option<String>, String, String, Option<String>, Option<String>, i32, String, String)> =
        sqlx::query_as("SELECT id, business_unit_code, keyword, platform, title, url, summary, score, status, created_at::text FROM listening_signals WHERE ($1::text IS NULL OR status=$1) AND ($2::text IS NULL OR business_unit_code=$2) ORDER BY score DESC, created_at DESC LIMIT 200")
            .bind(q.get("status"))
            .bind(q.get("business_unit_code"))
            .fetch_all(state.pool())
            .await?;
    let items: Vec<Value> = rows
        .into_iter()
        .map(|(id, bu, kw, platform, title, url, summary, score, status, at)| {
            json!({"id": id, "business_unit_code": bu, "keyword": kw, "platform": platform, "title": title, "url": url, "summary": summary, "score": score, "status": status, "created_at": at})
        })
        .collect();
    ok(json!({"items": items, "total": items.len()}))
}

/// 信号 → 选题
pub async fn signal_to_topic(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> ApiResult {
    let sig: Option<(String, Option<String>, Option<String>, String)> = sqlx::query_as(
        "SELECT title, business_unit_code, keyword, platform FROM listening_signals WHERE id=$1",
    )
    .bind(&id)
    .fetch_optional(state.pool())
    .await?;
    let (title, bu, kw, _platform) = sig.ok_or_else(|| AppError::not_found("信号不存在"))?;
    let tid = new_id();
    sqlx::query("INSERT INTO content_topics (id, business_unit_code, title, keywords, status) VALUES ($1,$2,$3,$4,'NEW')")
        .bind(&tid)
        .bind(bu.as_deref().unwrap_or("ERP"))
        .bind(&title)
        .bind(json!([kw.clone().unwrap_or_default()]))
        .execute(state.pool())
        .await?;
    sqlx::query("UPDATE listening_signals SET status='CONVERTED' WHERE id=$1")
        .bind(&id)
        .execute(state.pool())
        .await?;
    audit(
        state.pool(),
        &user.0.sub,
        "signal_to_topic",
        "listening_signal",
        &id,
        json!({"topic_id": tid}),
    )
    .await;
    ok(json!({"topic_id": tid, "title": title}))
}

/// 信号 → 线索
pub async fn signal_to_lead(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> ApiResult {
    let sig: Option<(String, Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT title, business_unit_code, url, summary FROM listening_signals WHERE id=$1",
    )
    .bind(&id)
    .fetch_optional(state.pool())
    .await?;
    let (title, bu, url, summary) = sig.ok_or_else(|| AppError::not_found("信号不存在"))?;
    let lid = new_id();
    sqlx::query("INSERT INTO leads (id, business_unit_code, name, source_channel, summary, source_content_id) VALUES ($1,$2,$3,'listening',$4,$5)")
        .bind(&lid)
        .bind(bu.clone().unwrap_or_default())
        .bind(truncate(&title, 100))
        .bind(truncate(&format!("{title} · {} · {}", url.clone().unwrap_or_default(), summary.unwrap_or_default()), 500))
        .bind(&id)
        .execute(state.pool())
        .await?;
    sqlx::query("UPDATE listening_signals SET status='CONVERTED' WHERE id=$1")
        .bind(&id)
        .execute(state.pool())
        .await?;
    audit(
        state.pool(),
        &user.0.sub,
        "signal_to_lead",
        "listening_signal",
        &id,
        json!({"lead_id": lid}),
    )
    .await;
    ok(json!({"lead_id": lid}))
}
