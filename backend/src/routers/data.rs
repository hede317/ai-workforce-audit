use crate::auth::{audit, new_id, AuthUser};
use crate::error::{ok, ApiResult, AppError};
use crate::state::AppState;
use crate::tools::{dispatch, ToolCtx};
use axum::extract::{Path, Query, State};
use axum::Json;
use serde_json::{json, Value};
use std::collections::HashMap;

// ---------- 记忆中心（四类记忆） ----------

pub async fn list_memories(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResult {
    let kind = q.get("kind");
    let status = q.get("status");
    let like = q
        .get("q")
        .map(|s| format!("%{}%", crate::tools::escape_like(s)));
    let rows: Vec<(String, String, String, String, Option<Value>, Option<String>, String, String)> =
        sqlx::query_as("SELECT id, kind, title, left(content,500), tags, source_task_id, status, updated_at::text FROM memories WHERE ($1::text IS NULL OR kind=$1) AND ($2::text IS NULL OR status=$2) AND ($3::text IS NULL OR title LIKE $3 ESCAPE '\\' OR content LIKE $3 ESCAPE '\\') ORDER BY updated_at DESC LIMIT 200")
            .bind(kind)
            .bind(status)
            .bind(&like)
            .fetch_all(state.pool())
            .await?;
    let items: Vec<Value> = rows
        .into_iter()
        .map(|(id, kind, title, content, tags, src, status, updated)| {
            json!({"id": id, "kind": kind, "title": title, "content": content, "tags": tags, "source_task_id": src, "status": status, "updated_at": updated})
        })
        .collect();
    ok(json!({"items": items, "total": items.len()}))
}

/// 用户手动创建记忆：用户是记忆的权威来源，ENTERPRISE 由用户创建直接生效
pub async fn create_memory(
    State(state): State<AppState>,
    user: crate::auth::AdminUser,
    Json(body): Json<Value>,
) -> ApiResult {
    let kind = body
        .get("kind")
        .and_then(|v| v.as_str())
        .unwrap_or("KNOWLEDGE");
    if !["ENTERPRISE", "USER", "TASK", "KNOWLEDGE"].contains(&kind) {
        return Err(AppError::bad_request(
            "kind 必须为 ENTERPRISE/USER/TASK/KNOWLEDGE",
        ));
    }
    let title = body
        .get("title")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::bad_request("缺少 title"))?;
    let content = body
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::bad_request("缺少 content"))?;
    let id = new_id();
    sqlx::query("INSERT INTO memories (id, kind, title, content, tags, status) VALUES ($1,$2,$3,$4,$5,'ACTIVE')")
        .bind(&id)
        .bind(kind)
        .bind(title)
        .bind(content)
        .bind(body.get("tags").cloned().unwrap_or(json!([])))
        .execute(state.pool())
        .await?;
    audit(
        state.pool(),
        &user.0.sub,
        "create",
        "memory",
        &id,
        json!({"kind": kind}),
    )
    .await;
    crate::tools::embed_memory(&state, &id, title, content).await;
    ok(json!({"id": id}))
}

/// 记忆修订：确认 PENDING→ACTIVE / 归档 / 修改内容（规范4.2：用户可查看、修改、删除）
pub async fn update_memory(
    State(state): State<AppState>,
    user: crate::auth::AdminUser,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult {
    if let Some(st) = body.get("status").and_then(|v| v.as_str()) {
        if !["ACTIVE", "PENDING", "ARCHIVED"].contains(&st) {
            return Err(AppError::bad_request(
                "status 只能为 ACTIVE/PENDING/ARCHIVED",
            ));
        }
    }
    let res = sqlx::query("UPDATE memories SET title=COALESCE($2,title), content=COALESCE($3,content), status=COALESCE($4,status), updated_at=now() WHERE id=$1")
        .bind(&id)
        .bind(body.get("title").and_then(|v| v.as_str()))
        .bind(body.get("content").and_then(|v| v.as_str()))
        .bind(body.get("status").and_then(|v| v.as_str()))
        .execute(state.pool())
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::not_found("记忆不存在"));
    }
    audit(state.pool(), &user.0.sub, "update", "memory", &id, json!({"fields": body.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()).unwrap_or_default()})).await;
    // 审计#9-2：修改后重算向量，保证语义检索可命中
    if let Some((title, content)) = {
        let row: Option<(String, String)> =
            sqlx::query_as("SELECT title, content FROM memories WHERE id=$1")
                .bind(&id)
                .fetch_optional(state.pool())
                .await?;
        row
    } {
        crate::tools::embed_memory(&state, &id, &title, &content).await;
    }
    ok(json!({"id": id, "updated": true}))
}

// ---------- 数据分析 ----------

pub async fn analytics_summary(State(state): State<AppState>, _user: AuthUser) -> ApiResult {
    // 审计#16：同表 FILTER 聚合，14 次查询收成 5 次
    let (t_total, t_active, t_done, t_failed, tokens): (i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), COUNT(*) FILTER (WHERE status IN ('Created','Running','Assigned','Planning')), COUNT(*) FILTER (WHERE status='Completed'), COUNT(*) FILTER (WHERE status='Failed'), COALESCE(SUM(tokens_used),0) FROM tasks",
    )
    .fetch_one(state.pool())
    .await
    .unwrap_or((0, 0, 0, 0, 0));
    let (m_total, m_pending): (i64, i64) = sqlx::query_as("SELECT COUNT(*) FILTER (WHERE status='ACTIVE'), COUNT(*) FILTER (WHERE status='PENDING') FROM memories")
        .fetch_one(state.pool()).await.unwrap_or((0, 0));
    // 二审P2-8：删除无效死查询；统计失败显式记日志而非伪装全零
    let c: (i64, i64, i64) = match sqlx::query_as("SELECT (SELECT COUNT(*) FROM content_topics), (SELECT COUNT(*) FROM content_assets), (SELECT COUNT(*) FROM content_assets WHERE status='APPROVED')")
        .fetch_one(state.pool())
        .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("analytics content 统计失败: {e}");
            (0, 0, 0)
        }
    };
    let (p_queue, p_published): (i64, i64) = sqlx::query_as("SELECT COUNT(*) FILTER (WHERE status='HUMAN_QUEUE'), COUNT(*) FILTER (WHERE status='PUBLISHED') FROM publishing_tasks")
        .fetch_one(state.pool()).await.unwrap_or((0, 0));
    let (l_total, l_new, l_won, agents_n, tools_n, ap_pending): (i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM leads), (SELECT COUNT(*) FROM leads WHERE lead_status='NEW'), (SELECT COUNT(*) FROM leads WHERE lead_status='WON'), (SELECT COUNT(*) FROM agents WHERE status='ACTIVE'), (SELECT COUNT(*) FROM tools WHERE status='ACTIVE'), (SELECT COUNT(*) FROM approvals WHERE status='PENDING')",
    )
    .fetch_one(state.pool())
    .await
    .unwrap_or((0, 0, 0, 0, 0, 0));
    let summary = json!({
        "tasks": { "total": t_total, "active": t_active, "completed": t_done, "failed": t_failed },
        "agents": agents_n,
        "tools": tools_n,
        "memories": { "total": m_total, "pending_review": m_pending },
        "content": { "topics": c.0, "assets": c.1, "approved": c.2 },
        "publishing": { "human_queue": p_queue, "published": p_published },
        "leads": { "total": l_total, "new": l_new, "won": l_won },
        "approvals_pending": ap_pending,
        "tokens_used": tokens,
    });
    ok(summary)
}

pub async fn daily_report(State(state): State<AppState>, _user: AuthUser) -> ApiResult {
    let ctx = ToolCtx {
        state: &state,
        task_id: "manual-report".into(),
        approved: true,
    };
    let value = dispatch("data.daily_report", json!({}), &ctx).await?;
    ok(value)
}

// ---------- 工作流（n8n 执行层） ----------

/// Agent/前端可请求 n8n 执行确定性工作流（规范10.1：n8n 保留为调度执行层）
pub async fn trigger_workflow(
    State(state): State<AppState>,
    user: crate::auth::AdminUser,
    Json(body): Json<Value>,
) -> ApiResult {
    let code = body
        .get("workflow_code")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::bad_request("缺少 workflow_code"))?;
    if !code
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        || code.len() > 64
    {
        return Err(AppError::bad_request(
            "workflow_code 仅允许字母/数字/下划线/连字符",
        ));
    }
    let base = state.0.cfg.n8n_base_url.trim_end_matches('/');
    if base.is_empty() {
        return Err(AppError::not_configured("n8n (N8N_BASE_URL)"));
    }
    let exec_id = new_id();
    sqlx::query("INSERT INTO workflow_executions (id, workflow_code, status, started_at) VALUES ($1,$2,'RUNNING',now())")
        .bind(&exec_id)
        .bind(code)
        .execute(state.pool())
        .await?;
    let url = format!("{base}/webhook/{code}");
    let resp = state
        .0
        .http
        .post(&url)
        .header("X-AWF-Token", &state.0.cfg.n8n_webhook_token)
        .json(&body.get("payload").cloned().unwrap_or(json!({})))
        .send()
        .await;
    match resp {
        Ok(r) => {
            let status_code = r.status().as_u16();
            let payload: Value = r.json().await.unwrap_or(json!({}));
            let wf_status = if (200..300).contains(&status_code) {
                "COMPLETED"
            } else {
                "FAILED"
            };
            sqlx::query(
                "UPDATE workflow_executions SET status=$2, finished_at=now(), error=$3 WHERE id=$1",
            )
            .bind(&exec_id)
            .bind(wf_status)
            .bind(if wf_status == "FAILED" {
                Some(format!("HTTP {status_code}"))
            } else {
                None
            })
            .execute(state.pool())
            .await?;
            audit(
                state.pool(),
                &user.0.sub,
                "workflow_trigger",
                "workflow_execution",
                &exec_id,
                json!({"code": code}),
            )
            .await;
            ok(json!({"execution_id": exec_id, "status": wf_status, "response": payload}))
        }
        Err(e) => {
            sqlx::query("UPDATE workflow_executions SET status='FAILED', finished_at=now(), error=$2 WHERE id=$1")
                .bind(&exec_id)
                .bind(format!("{e}"))
                .execute(state.pool())
                .await?;
            Err(AppError::internal(format!("n8n 调用失败: {e}")))
        }
    }
}

pub async fn list_executions(State(state): State<AppState>, _user: AuthUser) -> ApiResult {
    let rows: Vec<(String, String, Option<String>, String, Option<String>)> =
        sqlx::query_as("SELECT id, workflow_code, external_id, status, error FROM workflow_executions ORDER BY created_at DESC LIMIT 100")
            .fetch_all(state.pool())
            .await?;
    let items: Vec<Value> = rows
        .into_iter()
        .map(|(id, code, ext, status, error)| json!({"id": id, "workflow_code": code, "external_id": ext, "status": status, "error": error}))
        .collect();
    ok(json!({"items": items, "total": items.len()}))
}
