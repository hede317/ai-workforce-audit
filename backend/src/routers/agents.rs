use crate::auth::{audit, AdminUser, AuthUser};
use crate::error::{ok, AppError, ApiResult};
use crate::models::Task;
use crate::orchestrator;
use crate::state::AppState;
use crate::tools::TOOL_DEFS;
use axum::extract::{Path, Query, State};
use axum::Json;
use serde_json::{json, Value};
use std::collections::HashMap;

pub async fn list_agents(State(state): State<AppState>, _user: AuthUser) -> ApiResult {
    let rows: Vec<(String, String, String, String, Option<String>, Option<String>, Option<String>, Option<Value>, i32, i32, i32, i32, String)> =
        sqlx::query_as("SELECT id,code,name,role,responsibilities,system_prompt,model,tools,max_depth,max_tool_calls,max_tokens,max_seconds,status FROM agents ORDER BY code")
            .fetch_all(state.pool())
            .await?;
    let items: Vec<Value> = rows
        .into_iter()
        .map(|(id, code, name, role, resp, prompt, model, tools, md, mtc, mt, ms, status)| {
            json!({"id": id, "code": code, "name": name, "role": role, "responsibilities": resp, "system_prompt": prompt, "model": model, "tools": tools, "max_depth": md, "max_tool_calls": mtc, "max_tokens": mt, "max_seconds": ms, "status": status})
        })
        .collect();
    ok(json!({"items": items, "total": items.len()}))
}

pub async fn update_agent(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult {
    sqlx::query(
        "UPDATE agents SET name=COALESCE($2,name), role=COALESCE($3,role), responsibilities=COALESCE($4,responsibilities), system_prompt=COALESCE($5,system_prompt), model=COALESCE($6,model), tools=COALESCE($7,tools), status=COALESCE($8,status), max_depth=COALESCE($9,max_depth), max_tool_calls=COALESCE($10,max_tool_calls), max_tokens=COALESCE($11,max_tokens), max_seconds=COALESCE($12,max_seconds) WHERE id=$1",
    )
    .bind(&id)
    .bind(body.get("name").and_then(|v| v.as_str()))
    .bind(body.get("role").and_then(|v| v.as_str()))
    .bind(body.get("responsibilities").and_then(|v| v.as_str()))
    .bind(body.get("system_prompt").and_then(|v| v.as_str()))
    .bind(body.get("model").and_then(|v| v.as_str()))
    .bind(body.get("tools").cloned())
    .bind(body.get("status").and_then(|v| v.as_str()))
    .bind(body.get("max_depth").and_then(|v| v.as_i64()).map(|v| v as i32))
    .bind(body.get("max_tool_calls").and_then(|v| v.as_i64()).map(|v| v as i32))
    .bind(body.get("max_tokens").and_then(|v| v.as_i64()).map(|v| v as i32))
    .bind(body.get("max_seconds").and_then(|v| v.as_i64()).map(|v| v as i32))
    .execute(state.pool())
    .await?;
    audit(state.pool(), &admin.0.sub, "update", "agent", &id, json!({"fields": body.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()).unwrap_or_default()})).await;
    ok(json!({"id": id, "updated": true}))
}

pub async fn list_tools(State(state): State<AppState>, _user: AuthUser) -> ApiResult {
    let rows: Vec<(String, String, String, String, Option<String>, Option<Value>, String, i32)> =
        sqlx::query_as("SELECT id,code,name,category,description,input_schema,permission_level,timeout_seconds FROM tools ORDER BY category, code")
            .fetch_all(state.pool())
            .await?;
    let items: Vec<Value> = rows
        .into_iter()
        .map(|(id, code, name, category, description, schema, perm, timeout)| {
            json!({"id": id, "code": code, "name": name, "category": category, "description": description, "input_schema": schema, "permission_level": perm, "timeout_seconds": timeout})
        })
        .collect();
    ok(json!({"items": items, "total": items.len(), "registry_version": TOOL_DEFS.len()}))
}

// ---------- AI Commander ----------

pub async fn create_goal(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<Value>,
) -> ApiResult {
    let goal = body.get("goal").and_then(|v| v.as_str()).ok_or_else(|| AppError::bad_request("缺少 goal"))?;
    let bu = body.get("business_unit_code").and_then(|v| v.as_str()).map(|s| s.to_string());
    let result = orchestrator::create_goal(&state, goal, bu, &user.0.username).await?;
    audit(state.pool(), &user.0.sub, "create_goal", "task", result["root_id"].as_str().unwrap_or(""), json!({"goal": goal})).await;
    ok(result)
}

pub async fn get_goal(State(state): State<AppState>, _user: AuthUser, Path(id): Path<String>) -> ApiResult {
    let rows: Vec<Task> = sqlx::query_as("SELECT * FROM tasks WHERE id=$1 OR root_id=$1 ORDER BY depth, created_at")
        .bind(&id)
        .fetch_all(state.pool())
        .await?;
    if rows.is_empty() {
        return Err(AppError::not_found("目标不存在"));
    }
    let root = &rows[0];
    let children: Vec<Value> = rows[1..]
        .iter()
        .map(|t| task_json(t))
        .collect();
    ok(json!({"root": task_json(root), "children": children}))
}

pub async fn execute_goal(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<String>,
    body: Option<Json<Value>>,
) -> ApiResult {
    let sync = body
        .as_ref()
        .and_then(|b| b.0.get("sync").and_then(|v| v.as_bool()))
        .unwrap_or(true);
    if sync {
        let summary = orchestrator::execute_goal(&state, &id).await?;
        ok(summary)
    } else {
        let state2 = state.clone();
        let id2 = id.clone();
        tokio::spawn(async move {
            if let Err(e) = orchestrator::execute_goal(&state2, &id2).await {
                tracing::error!("后台执行目标失败: {e}");
            }
        });
        ok(json!({"started": true, "root_id": id}))
    }
}

// ---------- 任务 ----------

fn task_json(t: &Task) -> Value {
    json!({
        "id": t.id, "parent_id": t.parent_id, "root_id": t.root_id, "title": t.title,
        "description": t.description, "goal": t.goal, "business_unit_code": t.business_unit_code,
        "agent_code": t.agent_code, "status": t.status, "priority": t.priority, "depth": t.depth,
        "tool_hint": t.tool_hint, "args": t.args, "plan": t.plan, "result": t.result, "error": t.error,
        "requires_approval": t.requires_approval, "max_tool_calls": t.max_tool_calls,
        "max_tokens": t.max_tokens, "max_seconds": t.max_seconds,
        "tool_calls_used": t.tool_calls_used, "tokens_used": t.tokens_used,
        "started_at": t.started_at, "finished_at": t.finished_at, "created_at": t.created_at,
    })
}

pub async fn list_tasks(State(state): State<AppState>, _user: AuthUser, Query(q): Query<HashMap<String, String>>) -> ApiResult {
    let rows: Vec<Task> = if let Some(root) = q.get("root_id") {
        sqlx::query_as("SELECT * FROM tasks WHERE root_id=$1 ORDER BY depth, created_at DESC LIMIT 200").bind(root).fetch_all(state.pool()).await?
    } else if let Some(status) = q.get("status") {
        sqlx::query_as("SELECT * FROM tasks WHERE status=$1 ORDER BY created_at DESC LIMIT 100").bind(status).fetch_all(state.pool()).await?
    } else {
        sqlx::query_as("SELECT * FROM tasks ORDER BY created_at DESC LIMIT 100").fetch_all(state.pool()).await?
    };
    let items: Vec<Value> = rows.iter().map(task_json).collect();
    ok(json!({"items": items, "total": items.len()}))
}

pub async fn get_task(State(state): State<AppState>, _user: AuthUser, Path(id): Path<String>) -> ApiResult {
    let task: Task = sqlx::query_as("SELECT * FROM tasks WHERE id=$1")
        .bind(&id)
        .fetch_optional(state.pool())
        .await?
        .ok_or_else(|| AppError::not_found("任务不存在"))?;
    let events: Vec<(String, String, Option<Value>)> =
        sqlx::query_as("SELECT event, created_at::text, payload FROM task_events WHERE task_id=$1 ORDER BY created_at")
            .bind(&id)
            .fetch_all(state.pool())
            .await?;
    let mut tj = task_json(&task);
    tj["events"] = json!(events.into_iter().map(|(e, at, p)| json!({"event": e, "at": at, "payload": p})).collect::<Vec<_>>());
    ok(tj)
}

pub async fn retry_task(State(state): State<AppState>, user: AuthUser, Path(id): Path<String>) -> ApiResult {
    let res = sqlx::query("UPDATE tasks SET status='Created', error=NULL, started_at=NULL, finished_at=NULL WHERE id=$1 AND status='Failed'")
        .bind(&id)
        .execute(state.pool())
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::conflict("仅 Failed 状态的任务可重试"));
    }
    // 审计#7：重试不再清零预算累计（防无限重试突破预算），并立即执行 + 同步根任务
    sqlx::query("INSERT INTO task_events (id, task_id, event, payload) VALUES ($1,$2,'Retried',$3)")
        .bind(crate::auth::new_id())
        .bind(&id)
        .bind(json!({"by": user.0.username}))
        .execute(state.pool())
        .await?;
    let task: Task = sqlx::query_as("SELECT * FROM tasks WHERE id=$1").bind(&id).fetch_one(state.pool()).await?;
    crate::orchestrator::execute_task(&state, &id, task.requires_approval).await?;
    if let Some(root) = &task.root_id {
        crate::orchestrator::sync_root_status(&state, root).await?;
    }
    let fresh: Task = sqlx::query_as("SELECT * FROM tasks WHERE id=$1").bind(&id).fetch_one(state.pool()).await?;
    ok(json!({"id": id, "status": fresh.status, "error": fresh.error}))
}

pub async fn cancel_task(State(state): State<AppState>, _user: AuthUser, Path(id): Path<String>) -> ApiResult {
    let task: Task = sqlx::query_as("SELECT * FROM tasks WHERE id=$1").bind(&id).fetch_optional(state.pool()).await?
        .ok_or_else(|| AppError::not_found("任务不存在"))?;
    let res = sqlx::query("UPDATE tasks SET status='Cancelled', finished_at=now() WHERE id=$1 AND status IN ('Created','Waiting Approval','Planning','Assigned','Running')")
        .bind(&id)
        .execute(state.pool())
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::conflict("任务不在可取消状态"));
    }
    if let Some(root) = &task.root_id {
        crate::orchestrator::sync_root_status(&state, root).await?;
    }
    ok(json!({"id": id, "status": "Cancelled"}))
}

// ---------- 审批中心 ----------

pub async fn list_approvals(State(state): State<AppState>, _user: AuthUser, Query(q): Query<HashMap<String, String>>) -> ApiResult {
    let status = q.get("status").cloned().unwrap_or_else(|| "PENDING".into());
    // 审计#2：JOIN 任务携带工具与参数快照，审批人不再盲审
    let rows: Vec<(String, String, String, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<serde_json::Value>)> =
        sqlx::query_as("SELECT a.id, a.task_id, a.action, a.status, a.requested_by, a.decided_by, a.note, a.decided_at::text, t.tool_hint, t.agent_code, t.args FROM approvals a LEFT JOIN tasks t ON t.id=a.task_id WHERE a.status=$1 ORDER BY a.created_at DESC LIMIT 100")
            .bind(&status)
            .fetch_all(state.pool())
            .await?;
    let items: Vec<Value> = rows
        .into_iter()
        .map(|(id, task_id, action, st, req_by, dec_by, note, dec_at, tool, agent, args)| {
            json!({"id": id, "task_id": task_id, "action": action, "status": st, "requested_by": req_by, "decided_by": dec_by, "note": note, "decided_at": dec_at, "tool_hint": tool, "agent_code": agent, "args": args})
        })
        .collect();
    ok(json!({"items": items, "total": items.len()}))
}

pub async fn decide_approval(
    State(state): State<AppState>,
    user: crate::auth::ApproverUser,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult {
    let decision = body.get("decision").and_then(|v| v.as_str()).ok_or_else(|| AppError::bad_request("缺少 decision"))?;
    let note = body.get("note").and_then(|v| v.as_str()).unwrap_or("");
    let result = orchestrator::decide_approval(&state, &id, decision, &user.0.username, note).await?;
    audit(state.pool(), &user.0.sub, "approval", "approval", &id, json!({"decision": decision, "note": note})).await;
    ok(result)
}

/// 工具直调（仅 AUTO 级；APPROVAL 级必须走 Commander 任务+审批中心，红线不开口子）
pub async fn invoke_tool(
    State(state): State<AppState>,
    user: AuthUser,
    Path(code): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult {
    let perm = crate::tools::TOOL_DEFS.iter().find(|t| t.code == code).map(|t| t.permission_level);
    match perm {
        None => return Err(crate::error::AppError::not_found("未注册的工具")),
        Some("APPROVAL") => {
            return Err(crate::error::AppError::conflict(
                "APPROVAL_REQUIRED: 该工具须通过 AI Commander 下任务并经审批中心批准后执行",
            ))
        }
        _ => {}
    }
    // 审计#4：数据库注册表中被停用的工具同样拒绝直调
    let db_status: Option<(String,)> = sqlx::query_as("SELECT status FROM tools WHERE code=$1").bind(&code).fetch_optional(state.pool()).await?;
    if db_status.map(|(s,)| s != "ACTIVE").unwrap_or(false) {
        return Err(crate::error::AppError::conflict("该工具已在注册表中停用"));
    }
    let ctx = crate::tools::ToolCtx { state: &state, task_id: "manual-invoke".into(), approved: false };
    let value = crate::tools::dispatch(&code, body, &ctx).await?;
    audit(state.pool(), &user.0.sub, "invoke_tool", "tool", &code, json!({})).await;
    ok(value)
}
