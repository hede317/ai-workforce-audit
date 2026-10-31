use crate::auth::{audit, AuthUser, new_id};
use crate::error::{ok, AppError, ApiResult};
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::Json;
use serde_json::{json, Value};
use std::collections::HashMap;

// ---------- 选题 ----------

pub async fn list_topics(State(state): State<AppState>, _user: AuthUser, Query(q): Query<HashMap<String, String>>) -> ApiResult {
    // 审计#17：过滤条件下推 SQL，避免假空
    let rows: Vec<(String, Option<String>, String, Option<Value>, Option<String>, String, i32, String)> =
        sqlx::query_as("SELECT id, business_unit_code, title, keywords, audience, language, priority, status FROM content_topics WHERE ($1::text IS NULL OR business_unit_code=$1) AND ($2::text IS NULL OR status=$2) ORDER BY created_at DESC LIMIT $3 OFFSET $4")
            .bind(q.get("business_unit_code"))
            .bind(q.get("status"))
            .bind(q.get("page_size").and_then(|v| v.parse::<i64>().ok()).unwrap_or(200).clamp(1, 200))
            .bind(((q.get("page").and_then(|v| v.parse::<i64>().ok()).unwrap_or(1).clamp(1, 1000)) - 1) * q.get("page_size").and_then(|v| v.parse::<i64>().ok()).unwrap_or(200).clamp(1, 200))
            .fetch_all(state.pool())
            .await?;
    let items: Vec<Value> = rows
        .into_iter()
        .map(|(id, bu, title, kw, audience, lang, priority, status)| {
            json!({"id": id, "business_unit_code": bu, "title": title, "keywords": kw, "audience": audience, "language": lang, "priority": priority, "status": status})
        })
        .collect();
    ok(json!({"items": items, "total": items.len()}))
}

pub async fn create_topic(State(state): State<AppState>, user: AuthUser, Json(body): Json<Value>) -> ApiResult {
    let title = body.get("title").and_then(|v| v.as_str()).ok_or_else(|| AppError::bad_request("缺少 title"))?;
    let id = new_id();
    sqlx::query("INSERT INTO content_topics (id, business_unit_code, title, keywords, audience, market, language, priority, status) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,'NEW')")
        .bind(&id)
        .bind(body.get("business_unit_code").and_then(|v| v.as_str()))
        .bind(title)
        .bind(body.get("keywords").cloned())
        .bind(body.get("audience").and_then(|v| v.as_str()))
        .bind(body.get("market").and_then(|v| v.as_str()))
        .bind(body.get("language").and_then(|v| v.as_str()).unwrap_or("zh"))
        .bind(body.get("priority").and_then(|v| v.as_i64()).unwrap_or(5) as i32)
        .execute(state.pool())
        .await?;
    audit(state.pool(), &user.0.sub, "create", "content_topic", &id, json!({})).await;
    ok(json!({"id": id, "status": "NEW"}))
}

// ---------- 内容 ----------

pub async fn list_contents(State(state): State<AppState>, _user: AuthUser) -> ApiResult {
    let rows: Vec<(String, Option<String>, String, String, String, String, String, Option<Value>)> =
        sqlx::query_as("SELECT id, topic_id, business_unit_code, content_type, title, language, status, seo_metadata FROM content_assets ORDER BY created_at DESC LIMIT 200")
            .fetch_all(state.pool())
            .await?;
    let items: Vec<Value> = rows
        .into_iter()
        .map(|(id, topic_id, bu, ctype, title, lang, status, seo)| {
            json!({"id": id, "topic_id": topic_id, "business_unit_code": bu, "content_type": ctype, "title": title, "language": lang, "status": status, "seo_metadata": seo})
        })
        .collect();
    ok(json!({"items": items, "total": items.len()}))
}

pub async fn create_content(State(state): State<AppState>, user: AuthUser, Json(body): Json<Value>) -> ApiResult {
    let title = body.get("title").and_then(|v| v.as_str()).ok_or_else(|| AppError::bad_request("缺少 title"))?;
    let body_text = body.get("body").and_then(|v| v.as_str()).unwrap_or_default();
    let id = new_id();
    sqlx::query("INSERT INTO content_assets (id, topic_id, business_unit_code, content_type, title, body, language, status) VALUES ($1,$2,$3,$4,$5,$6,$7,'DRAFT')")
        .bind(&id)
        .bind(body.get("topic_id").and_then(|v| v.as_str()))
        .bind(body.get("business_unit_code").and_then(|v| v.as_str()))
        .bind(body.get("content_type").and_then(|v| v.as_str()).unwrap_or("article"))
        .bind(title)
        .bind(body_text)
        .bind(body.get("language").and_then(|v| v.as_str()).unwrap_or("zh"))
        .execute(state.pool())
        .await?;
    audit(state.pool(), &user.0.sub, "create", "content_asset", &id, json!({})).await;
    ok(json!({"id": id, "status": "DRAFT"}))
}

pub async fn approve_content(State(state): State<AppState>, user: crate::auth::EditorUser, Path(id): Path<String>) -> ApiResult {
    let res = sqlx::query("UPDATE content_assets SET status='APPROVED' WHERE id=$1 AND status IN ('DRAFT','IN_REVIEW')")
        .bind(&id)
        .execute(state.pool())
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::conflict("内容不在待审状态"));
    }
    audit(state.pool(), &user.0.sub, "approve", "content_asset", &id, json!({})).await;
    ok(json!({"id": id, "status": "APPROVED"}))
}

/// 内容发布：默认进入人工发布队列（HUMAN_QUEUE），记录可追溯
pub async fn publish_content(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult {
    let row: Option<(String, String, Option<Value>)> =
        sqlx::query_as("SELECT title, status, seo_metadata FROM content_assets WHERE id=$1")
            .bind(&id)
            .fetch_optional(state.pool())
            .await?;
    let (title, status, _seo) = row.ok_or_else(|| AppError::not_found("内容不存在"))?;
    if status != "APPROVED" {
        return Err(AppError::conflict("内容未审批，禁止发布（红线：所有公开内容必须人工审批）"));
    }
    let platform = body.get("platform").and_then(|v| v.as_str()).unwrap_or("website");
    // 审计红线2补强：同一内容存在进行中的发布任务时禁止重复生成
    let dup: Option<(String,)> =
        sqlx::query_as("SELECT id FROM publishing_tasks WHERE content_asset_id=$1 AND status IN ('HUMAN_QUEUE','DRAFT','SCHEDULED','PUBLISHING') LIMIT 1")
            .bind(&id)
            .fetch_optional(state.pool())
            .await?;
    if dup.is_some() {
        return Err(AppError::conflict("该内容已有进行中的发布任务，请先完成或取消再新建"));
    }
    let pt_id = new_id();
    let payload = json!({"title": title, "content_asset_id": id, "requested_by": user.0.username, "mode": "human_upload", "checklist": ["核对账号授权", "AI内容标注", "发布后回填链接到 publishing_results"]});
    sqlx::query("INSERT INTO publishing_tasks (id, content_asset_id, platform, status, payload) VALUES ($1,$2,$3,'HUMAN_QUEUE',$4)")
        .bind(&pt_id)
        .bind(&id)
        .bind(platform)
        .bind(&payload)
        .execute(state.pool())
        .await?;
    audit(state.pool(), &user.0.sub, "publish", "publishing_task", &pt_id, json!({"platform": platform})).await;
    ok(json!({"publishing_task_id": pt_id, "platform": platform, "status": "HUMAN_QUEUE"}))
}

// ---------- 线索 CRM ----------

pub async fn list_leads(State(state): State<AppState>, _user: AuthUser, Query(q): Query<HashMap<String, String>>) -> ApiResult {
    let rows: Vec<(String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, String, String)> =
        sqlx::query_as("SELECT id, business_unit_code, name, email, source_channel, country, lead_status, consent_status FROM leads WHERE ($1::text IS NULL OR business_unit_code=$1) AND ($2::text IS NULL OR lead_status=$2) ORDER BY created_at DESC LIMIT $3 OFFSET $4")
            .bind(q.get("business_unit_code"))
            .bind(q.get("lead_status"))
            .bind(q.get("page_size").and_then(|v| v.parse::<i64>().ok()).unwrap_or(200).clamp(1, 200))
            .bind(((q.get("page").and_then(|v| v.parse::<i64>().ok()).unwrap_or(1).clamp(1, 1000)) - 1) * q.get("page_size").and_then(|v| v.parse::<i64>().ok()).unwrap_or(200).clamp(1, 200))
            .fetch_all(state.pool())
            .await?;
    let items: Vec<Value> = rows
        .into_iter()
        .map(|(id, bu, name, email, channel, country, status, consent)| {
            json!({"id": id, "business_unit_code": bu, "name": name, "email": email, "source_channel": channel, "country": country, "lead_status": status, "consent_status": consent})
        })
        .collect();
    ok(json!({"items": items, "total": items.len()}))
}

pub async fn create_lead(State(state): State<AppState>, _user: AuthUser, Json(body): Json<Value>) -> ApiResult {
    let email = body.get("email").and_then(|v| v.as_str()).map(|e| e.trim().to_lowercase()).filter(|e| !e.is_empty());
    if let Some(e) = &email {
        let dup: Option<(String,)> = sqlx::query_as("SELECT id FROM leads WHERE lower(btrim(email))=$1").bind(e).fetch_optional(state.pool()).await?;
        if dup.is_some() {
            return Err(AppError::conflict("该邮箱线索已存在"));
        }
    }
    let id = new_id();
    sqlx::query("INSERT INTO leads (id, business_unit_code, name, email, phone, country, source_channel, source_content_id, consent_status, summary, metadata) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)")
        .bind(&id)
        .bind(body.get("business_unit_code").and_then(|v| v.as_str()))
        .bind(body.get("name").and_then(|v| v.as_str()))
        .bind(email)
        .bind(body.get("phone").and_then(|v| v.as_str()))
        .bind(body.get("country").and_then(|v| v.as_str()))
        .bind(body.get("source_channel").and_then(|v| v.as_str()).unwrap_or("website"))
        .bind(body.get("source_content_id").and_then(|v| v.as_str()))
        .bind(body.get("consent_status").and_then(|v| v.as_str()).unwrap_or("UNKNOWN"))
        .bind(body.get("summary").and_then(|v| v.as_str()))
        .bind(body.get("metadata").cloned())
        .execute(state.pool())
        .await?;
    sqlx::query("INSERT INTO lead_events (id, lead_id, event_type, note) VALUES ($1,$2,'inquiry',$3)")
        .bind(new_id())
        .bind(&id)
        .bind(body.get("summary").and_then(|v| v.as_str()).unwrap_or("表单提交"))
        .execute(state.pool())
        .await?;
    ok(json!({"id": id, "lead_status": "NEW"}))
}

pub async fn list_lead_events(State(state): State<AppState>, _user: AuthUser, Path(id): Path<String>) -> ApiResult {
    let rows: Vec<(String, String, Option<String>, String)> =
        sqlx::query_as("SELECT id, event_type, note, created_at::text FROM lead_events WHERE lead_id=$1 ORDER BY created_at")
            .bind(&id)
            .fetch_all(state.pool())
            .await?;
    let items: Vec<Value> = rows
        .into_iter()
        .map(|(eid, etype, note, at)| json!({"id": eid, "event_type": etype, "note": note, "at": at}))
        .collect();
    ok(json!({"items": items}))
}

pub async fn add_lead_event(State(state): State<AppState>, _user: AuthUser, Path(id): Path<String>, Json(body): Json<Value>) -> ApiResult {
    let event_type = body.get("event_type").and_then(|v| v.as_str()).ok_or_else(|| AppError::bad_request("缺少 event_type"))?;
    if !["inquiry", "reply", "followup", "complaint", "deal", "lost"].contains(&event_type) {
        return Err(AppError::bad_request("event_type 必须为 inquiry/reply/followup/complaint/deal/lost"));
    }
    let eid = new_id();
    sqlx::query("INSERT INTO lead_events (id, lead_id, event_type, note) VALUES ($1,$2,$3,$4)")
        .bind(&eid)
        .bind(&id)
        .bind(event_type)
        .bind(body.get("note").and_then(|v| v.as_str()))
        .execute(state.pool())
        .await?;
    if let Some(new_status) = body.get("lead_status").and_then(|v| v.as_str()) {
        sqlx::query("UPDATE leads SET lead_status=$2 WHERE id=$1").bind(&id).bind(new_status).execute(state.pool()).await?;
    }
    ok(json!({"id": eid}))
}

// ---------- 营销活动 ----------

pub async fn list_campaigns(State(state): State<AppState>, _user: AuthUser) -> ApiResult {
    let rows: Vec<(String, Option<String>, String, Option<String>, f64, f64, String)> =
        sqlx::query_as("SELECT id, business_unit_code, name, channel, budget, spend, status FROM campaigns ORDER BY created_at DESC LIMIT 100")
            .fetch_all(state.pool())
            .await?;
    let items: Vec<Value> = rows
        .into_iter()
        .map(|(id, bu, name, channel, budget, spend, status)| {
            json!({"id": id, "business_unit_code": bu, "name": name, "channel": channel, "budget": budget, "spend": spend, "status": status})
        })
        .collect();
    ok(json!({"items": items, "total": items.len()}))
}

pub async fn create_campaign(State(state): State<AppState>, user: AuthUser, Json(body): Json<Value>) -> ApiResult {
    let name = body.get("name").and_then(|v| v.as_str()).ok_or_else(|| AppError::bad_request("缺少 name"))?;
    let id = new_id();
    sqlx::query("INSERT INTO campaigns (id, business_unit_code, name, channel, budget, status, utm) VALUES ($1,$2,$3,$4,$5,$6,$7)")
        .bind(&id)
        .bind(body.get("business_unit_code").and_then(|v| v.as_str()))
        .bind(name)
        .bind(body.get("channel").and_then(|v| v.as_str()))
        .bind(body.get("budget").and_then(|v| v.as_f64()).unwrap_or(0.0))
        .bind(body.get("status").and_then(|v| v.as_str()).unwrap_or("DRAFT"))
        .bind(body.get("utm").cloned())
        .execute(state.pool())
        .await?;
    audit(state.pool(), &user.0.sub, "create", "campaign", &id, json!({"name": name})).await;
    ok(json!({"id": id}))
}

// ---------- 发布中心（内容ID追溯闭环的最后一环） ----------

pub async fn list_publishing(State(state): State<AppState>, _user: AuthUser) -> ApiResult {
    let tasks: Vec<(String, Option<String>, String, Option<String>, String, Option<Value>, String)> =
        sqlx::query_as("SELECT id, content_asset_id, platform, account, status, payload, created_at::text FROM publishing_tasks ORDER BY created_at DESC LIMIT 100")
            .fetch_all(state.pool())
            .await?;
    let ids: Vec<String> = tasks.iter().map(|(id, ..)| id.clone()).collect();
    let results: Vec<(String, Option<String>, Option<String>, Option<Value>, Option<String>)> =
        sqlx::query_as("SELECT publishing_task_id, platform_post_id, published_url, metrics, published_at::text FROM publishing_results WHERE publishing_task_id = ANY($1)")
            .bind(&ids)
            .fetch_all(state.pool())
            .await?;
    let items: Vec<Value> = tasks
        .into_iter()
        .map(|(id, ca, platform, account, status, payload, created)| {
            let rs: Vec<Value> = results
                .iter()
                .filter(|(tid, _, _, _, _)| *tid == id)
                .map(|(_, pid, url, metrics, at)| json!({"platform_post_id": pid, "published_url": url, "metrics": metrics, "published_at": at}))
                .collect();
            json!({"id": id, "content_asset_id": ca, "platform": platform, "account": account, "status": status, "payload": payload, "created_at": created, "results": rs})
        })
        .collect();
    ok(json!({"items": items, "total": items.len()}))
}

/// 回填发布结果：人工发布（或API发布）后记录链接与表现 → 任务转 PUBLISHED
pub async fn record_publishing_result(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult {
    let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM publishing_tasks WHERE id=$1")
        .bind(&id)
        .fetch_optional(state.pool())
        .await?;
    if exists.is_none() {
        return Err(AppError::not_found("发布任务不存在"));
    }
    let rid = new_id();
    sqlx::query("INSERT INTO publishing_results (id, publishing_task_id, platform_post_id, published_url, published_at, metrics) VALUES ($1,$2,$3,$4,now(),$5)")
        .bind(&rid)
        .bind(&id)
        .bind(body.get("platform_post_id").and_then(|v| v.as_str()))
        .bind(body.get("published_url").and_then(|v| v.as_str()))
        .bind(body.get("metrics").cloned())
        .execute(state.pool())
        .await?;
    sqlx::query("UPDATE publishing_tasks SET status='PUBLISHED' WHERE id=$1")
        .bind(&id)
        .execute(state.pool())
        .await?;
    audit(state.pool(), &user.0.sub, "publish_result", "publishing_task", &id, json!({"url": body.get("published_url")})).await;
    ok(json!({"id": rid, "publishing_task_id": id, "status": "PUBLISHED"}))
}

// ---------- 选品中心（ECOM） ----------

pub async fn list_products(State(state): State<AppState>, _user: AuthUser, Query(q): Query<HashMap<String, String>>) -> ApiResult {
    let rows: Vec<(String, Option<String>, String, Option<String>, f64, f64, f64, i32, i32, String, Option<String>)> =
        sqlx::query_as("SELECT id, business_unit_code, title, source_url, price, est_cost, est_margin_pct, demand_score, competition_score, status, decision_note FROM product_candidates ORDER BY updated_at DESC LIMIT 200")
            .fetch_all(state.pool())
            .await?;
    let mut items: Vec<Value> = rows
        .into_iter()
        .map(|(id, bu, title, url, price, cost, margin, demand, comp, status, note)| {
            json!({"id": id, "business_unit_code": bu, "title": title, "source_url": url, "price": price, "est_cost": cost, "est_margin_pct": margin, "demand_score": demand, "competition_score": comp, "status": status, "decision_note": note})
        })
        .collect();
    if let Some(want) = q.get("status") {
        items.retain(|i| i["status"].as_str() == Some(want.as_str()));
    }
    ok(json!({"items": items, "total": items.len()}))
}

pub async fn create_product(State(state): State<AppState>, user: AuthUser, Json(body): Json<Value>) -> ApiResult {
    let title = body.get("title").and_then(|v| v.as_str()).ok_or_else(|| AppError::bad_request("缺少 title"))?;
    let id = new_id();
    sqlx::query("INSERT INTO product_candidates (id, business_unit_code, title, source_url, price) VALUES ($1,$2,$3,$4,$5)")
        .bind(&id)
        .bind(body.get("business_unit_code").and_then(|v| v.as_str()))
        .bind(title)
        .bind(body.get("source_url").and_then(|v| v.as_str()))
        .bind(body.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0))
        .execute(state.pool())
        .await?;
    audit(state.pool(), &user.0.sub, "create", "product_candidate", &id, json!({})).await;
    ok(json!({"id": id, "status": "PENDING"}))
}

/// 状态流转：PENDING→ANALYZED→APPROVED→LISTED（上架=敏感动作，仅 ADMIN）
pub async fn update_product(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult {
    if let Some(target) = body.get("status").and_then(|v| v.as_str()) {
        if !["PENDING", "ANALYZED", "APPROVED", "REJECTED", "LISTED"].contains(&target) {
            return Err(AppError::bad_request("非法状态"));
        }
        if target == "LISTED" && user.0.role != "ADMIN" {
            return Err(AppError::forbidden("商品上架是敏感动作，仅 ADMIN 可执行"));
        }
        // 审计红线4补强：状态机不可跳步
        let cur: (String,) = sqlx::query_as("SELECT status FROM product_candidates WHERE id=$1").bind(&id).fetch_one(state.pool()).await?;
        let allowed: &[&str] = match cur.0.as_str() {
            "PENDING" => &["ANALYZED", "REJECTED"],
            "ANALYZED" => &["APPROVED", "REJECTED"],
            "APPROVED" => &["LISTED", "REJECTED"],
            _ => &[],
        };
        if !allowed.contains(&target) {
            return Err(AppError::conflict(format!("状态机不允许从 {} 跳到 {}（须走 ANALYZED→APPROVED→LISTED）", cur.0, target)));
        }
    }
    let res = sqlx::query("UPDATE product_candidates SET status=COALESCE($2,status), decision_note=COALESCE($3,decision_note), price=COALESCE($4,price), updated_at=now() WHERE id=$1")
        .bind(&id)
        .bind(body.get("status").and_then(|v| v.as_str()))
        .bind(body.get("decision_note").and_then(|v| v.as_str()))
        .bind(body.get("price").and_then(|v| v.as_f64()))
        .execute(state.pool())
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::not_found("选品不存在"));
    }
    audit(state.pool(), &user.0.sub, "update", "product_candidate", &id, json!({"fields": body.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()).unwrap_or_default()})).await;
    ok(json!({"id": id, "updated": true}))
}

/// 内容详情（含正文）
pub async fn get_content(State(state): State<AppState>, _user: AuthUser, Path(id): Path<String>) -> ApiResult {
    let row: Option<(String, Option<String>, Option<String>, String, String, Option<String>, Option<Value>)> =
        sqlx::query_as("SELECT title, content_type, body, language, status, business_unit_code, seo_metadata FROM content_assets WHERE id=$1")
            .bind(&id)
            .fetch_optional(state.pool())
            .await?;
    let (title, ctype, body_text, lang, status, bu, seo) = row.ok_or_else(|| AppError::not_found("内容不存在"))?;
    ok(json!({"id": id, "title": title, "content_type": ctype, "body": body_text, "language": lang, "status": status, "business_unit_code": bu, "seo_metadata": seo}))
}
