use crate::auth::{audit, new_id, AuthUser};
use crate::error::{ok, AppError, ApiResult};
use crate::state::AppState;
use crate::tools::{dispatch, ToolCtx};
use axum::extract::{Path, Query, State};
use axum::Json;
use serde_json::{json, Value};
use std::collections::HashMap;

// ---------- 记忆中心（四类记忆） ----------

pub async fn list_memories(State(state): State<AppState>, _user: AuthUser, Query(q): Query<HashMap<String, String>>) -> ApiResult {
    let kind = q.get("kind");
    let status = q.get("status");
    let like = q.get("q").map(|s| format!("%{}%", s.replace('%', "")));
    let rows: Vec<(String, String, String, String, Option<Value>, Option<String>, String, String)> =
        sqlx::query_as("SELECT id, kind, title, left(content,500), tags, source_task_id, status, updated_at::text FROM memories WHERE ($1::text IS NULL OR kind=$1) AND ($2::text IS NULL OR status=$2) AND ($3::text IS NULL OR title LIKE $3 OR content LIKE $3) ORDER BY updated_at DESC LIMIT 200")
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
pub async fn create_memory(State(state): State<AppState>, user: AuthUser, Json(body): Json<Value>) -> ApiResult {
    let kind = body.get("kind").and_then(|v| v.as_str()).unwrap_or("KNOWLEDGE");
    if !["ENTERPRISE", "USER", "TASK", "KNOWLEDGE"].contains(&kind) {
        return Err(AppError::bad_request("kind 必须为 ENTERPRISE/USER/TASK/KNOWLEDGE"));
    }
    let title = body.get("title").and_then(|v| v.as_str()).ok_or_else(|| AppError::bad_request("缺少 title"))?;
    let content = body.get("content").and_then(|v| v.as_str()).ok_or_else(|| AppError::bad_request("缺少 content"))?;
    let id = new_id();
    sqlx::query("INSERT INTO memories (id, kind, title, content, tags, status) VALUES ($1,$2,$3,$4,$5,'ACTIVE')")
        .bind(&id)
        .bind(kind)
        .bind(title)
        .bind(content)
        .bind(body.get("tags").cloned().unwrap_or(json!([])))
        .execute(state.pool())
        .await?;
    audit(state.pool(), &user.0.sub, "create", "memory", &id, json!({"kind": kind})).await;
    ok(json!({"id": id}))
}

/// 记忆修订：确认 PENDING→ACTIVE / 归档 / 修改内容（规范4.2：用户可查看、修改、删除）
pub async fn update_memory(State(state): State<AppState>, user: AuthUser, Path(id): Path<String>, Json(body): Json<Value>) -> ApiResult {
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
    ok(json!({"id": id, "updated": true}))
}

// ---------- 数据分析 ----------

pub async fn analytics_summary(State(state): State<AppState>, _user: AuthUser) -> ApiResult {
    let one = |sql: &str| {
        let pool = state.pool().clone();
        let sql = sql.to_string();
        async move { sqlx::query_as::<_, (i64,)>(&sql).fetch_one(&pool).await.map(|r| r.0).unwrap_or(0) }
    };
    let summary = json!({
        "tasks": {
            "total": one("SELECT COUNT(*) FROM tasks").await,
            "active": one("SELECT COUNT(*) FROM tasks WHERE status IN ('Created','Running','Assigned','Planning')").await,
            "completed": one("SELECT COUNT(*) FROM tasks WHERE status='Completed'").await,
            "failed": one("SELECT COUNT(*) FROM tasks WHERE status='Failed'").await,
        },
        "agents": one("SELECT COUNT(*) FROM agents WHERE status='ACTIVE'").await,
        "tools": one("SELECT COUNT(*) FROM tools WHERE status='ACTIVE'").await,
        "memories": {
            "total": one("SELECT COUNT(*) FROM memories WHERE status='ACTIVE'").await,
            "pending_review": one("SELECT COUNT(*) FROM memories WHERE status='PENDING'").await,
        },
        "content": {
            "topics": one("SELECT COUNT(*) FROM content_topics").await,
            "assets": one("SELECT COUNT(*) FROM content_assets").await,
            "approved": one("SELECT COUNT(*) FROM content_assets WHERE status='APPROVED'").await,
        },
        "publishing": {
            "human_queue": one("SELECT COUNT(*) FROM publishing_tasks WHERE status='HUMAN_QUEUE'").await,
            "published": one("SELECT COUNT(*) FROM publishing_tasks WHERE status='PUBLISHED'").await,
        },
        "leads": {
            "total": one("SELECT COUNT(*) FROM leads").await,
            "new": one("SELECT COUNT(*) FROM leads WHERE lead_status='NEW'").await,
            "won": one("SELECT COUNT(*) FROM leads WHERE lead_status='WON'").await,
        },
        "approvals_pending": one("SELECT COUNT(*) FROM approvals WHERE status='PENDING'").await,
        "tokens_used": one("SELECT COALESCE(SUM(tokens_used),0) FROM tasks").await,
    });
    ok(summary)
}

pub async fn daily_report(State(state): State<AppState>, _user: AuthUser) -> ApiResult {
    let ctx = ToolCtx { state: &state, task_id: "manual-report".into(), approved: true };
    let value = dispatch("data.daily_report", json!({}), &ctx).await?;
    ok(value)
}

// ---------- 工作流（n8n 执行层） ----------

/// Agent/前端可请求 n8n 执行确定性工作流（规范10.1：n8n 保留为调度执行层）
pub async fn trigger_workflow(State(state): State<AppState>, user: AuthUser, Json(body): Json<Value>) -> ApiResult {
    let code = body.get("workflow_code").and_then(|v| v.as_str()).ok_or_else(|| AppError::bad_request("缺少 workflow_code"))?;
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
            let wf_status = if (200..300).contains(&status_code) { "COMPLETED" } else { "FAILED" };
            sqlx::query("UPDATE workflow_executions SET status=$2, finished_at=now(), error=$3 WHERE id=$1")
                .bind(&exec_id)
                .bind(wf_status)
                .bind(if wf_status == "FAILED" { Some(format!("HTTP {status_code}")) } else { None })
                .execute(state.pool())
                .await?;
            audit(state.pool(), &user.0.sub, "workflow_trigger", "workflow_execution", &exec_id, json!({"code": code})).await;
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
