use crate::ai::{extract_json, AiProvider, ChatMessage};
use crate::auth::new_id;
use crate::error::AppError;
use crate::models::{Agent, Task};
use crate::state::AppState;
use crate::tools::{dispatch, is_valid_tool, ToolCtx};
use serde_json::{json, Value};
use std::time::Duration;

const TERMINAL: [&str; 5] = [
    "Completed",
    "Failed",
    "Cancelled",
    "Rejected",
    "PartialFailed",
];

struct PlannedSubtask {
    title: String,
    agent_code: String,
    description: String,
    tool_hint: Option<String>,
    args: Value,
    requires_approval: bool,
}

/// 审批快照哈希（serde_json Map 为有序序列化，输出稳定）
fn args_hash(v: &Value) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(v.to_string());
    format!("{:x}", h.finalize())
}

fn truncate(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

pub(crate) async fn add_task_event(state: &AppState, task_id: &str, event: &str, payload: Value) {
    add_event(state, task_id, event, payload).await;
}

async fn add_event(state: &AppState, task_id: &str, event: &str, payload: Value) {
    let _ =
        sqlx::query("INSERT INTO task_events (id, task_id, event, payload) VALUES ($1,$2,$3,$4)")
            .bind(new_id())
            .bind(task_id)
            .bind(event)
            .bind(payload)
            .execute(state.pool())
            .await;
}

/// 二审P1-3：审批创建即固化工具+参数快照+哈希（批准时校验未被篡改）
pub(crate) async fn create_approval(
    state: &AppState,
    task: &Task,
    requested_by: &str,
) -> Result<(), AppError> {
    let args = task.args.clone().unwrap_or(json!({}));
    sqlx::query("INSERT INTO approvals (id, task_id, action, status, requested_by, tool_code, args_snapshot, args_hash) VALUES ($1,$2,$3,'PENDING',$4,$5,$6,$7)")
        .bind(new_id())
        .bind(&task.id)
        .bind(truncate(&task.title, 200))
        .bind(requested_by)
        .bind(task.tool_hint.as_deref())
        .bind(&args)
        .bind(args_hash(&args))
        .execute(state.pool())
        .await?;
    Ok(())
}

async fn set_task_status(state: &AppState, task_id: &str, status: &str) {
    let _ = sqlx::query("UPDATE tasks SET status=$2 WHERE id=$1")
        .bind(task_id)
        .bind(status)
        .execute(state.pool())
        .await;
}

// ---------- 目标拆解（Task Planner） ----------

/// 离线兜底计划：无需任何外部依赖即可拆解目标（AI 未配置时系统仍然可用）
fn fallback_plan(goal: &str, bu_code: &str, bu_website: &str) -> Vec<PlannedSubtask> {
    let g = truncate(goal.trim(), 40);
    vec![
        PlannedSubtask {
            title: format!("检索与「{g}」相关的企业记忆和历史经验"),
            agent_code: "market.research".into(),
            description: format!("检索长期记忆中与目标相关的企业资料、历史任务与经验教训：{goal}"),
            tool_hint: Some("memory.search".into()),
            args: json!({"q": truncate(goal, 20)}),
            requires_approval: false,
        },
        PlannedSubtask {
            title: format!("为「{g}」生成内容选题"),
            agent_code: "content".into(),
            description: format!("围绕经营目标生成5个可生产的内容选题：{goal}"),
            tool_hint: Some("content.topics_template".into()),
            args: json!({"goal": goal, "business_unit_code": bu_code}),
            requires_approval: false,
        },
        PlannedSubtask {
            title: "生成当日经营日报".into(),
            agent_code: "data.analyst".into(),
            description: "汇总当日任务/内容/线索数据，产出经营日报".into(),
            tool_hint: Some("data.daily_report".into()),
            args: json!({}),
            requires_approval: false,
        },
        PlannedSubtask {
            title: format!("准备社媒发布包（{bu_code}）"),
            agent_code: "social.media".into(),
            description: format!("为目标准备社媒人工发布包。业务官网：{bu_website}"),
            tool_hint: Some("social.package_upload".into()),
            args: json!({"platform": "tiktok", "title": truncate(goal, 40)}),
            requires_approval: true,
        },
    ]
}

/// AI 拆解：CEO Agent 生成任务计划（JSON）
async fn ai_plan(
    state: &AppState,
    goal: &str,
    bu_codes: &[String],
    agents: &[Agent],
) -> Result<(Vec<PlannedSubtask>, Option<String>), AppError> {
    let agent_list = agents
        .iter()
        .map(|a| {
            format!(
                "- {}（{}：{}）",
                a.code,
                a.name,
                a.responsibilities.as_deref().unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let tool_list = crate::tools::TOOL_DEFS
        .iter()
        .map(|t| {
            format!(
                "- {}（{}，权限:{}）",
                t.code, t.description, t.permission_level
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let provider = AiProvider {
        base_url: &state.0.cfg.ai_base_url,
        api_key: &state.0.cfg.ai_api_key,
        model: &state.0.cfg.ai_model,
        http: &state.0.http,
    };
    let reply = provider.chat(
        &[
            ChatMessage {
                role: "system".into(),
                content: "你是老何，HedeYou AI Workforce 的幕僚长，直接对德友（用户）负责。你听到德友的目标后，把它拆解为可执行任务并分配给专业 Agent。只输出 JSON，不要输出其他文字。".into(),
            },
            ChatMessage {
                role: "user".into(),
                content: format!(
                    "经营目标：{goal}\n\n可用业务单元：{}\n\n可用Agent：\n{agent_list}\n\n可用工具（tool 字段只能从这里选或留空由Agent自行用AI完成）：\n{tool_list}\n\n拆解为2-6个子任务，输出JSON：\n{{\"business_unit_code\":\"业务单元代码\",\"subtasks\":[{{\"title\":\"任务名\",\"agent_code\":\"agent代码\",\"description\":\"给Agent的具体指令\",\"tool\":\"工具代码或省略\",\"args\":{{}} ,\"requires_approval\":false}}]}}\n规则：对外发布/发邮件/联系客户/改价/创建广告的任务 requires_approval 必须 true。",
                    bu_codes.join("/")
                ),
            },
        ],
        2000,
    )
    .await?;
    let parsed =
        extract_json(&reply.text).ok_or_else(|| AppError::internal("CEO 拆解结果不是有效 JSON"))?;
    let bu = parsed
        .get("business_unit_code")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let mut out = Vec::new();
    if let Some(items) = parsed.get("subtasks").and_then(|v| v.as_array()) {
        for it in items {
            let agent_code = it
                .get("agent_code")
                .and_then(|v| v.as_str())
                .unwrap_or("data.analyst")
                .to_string();
            if !agents.iter().any(|a| a.code == agent_code) {
                continue; // 无效Agent直接丢弃，防幻觉
            }
            let tool = it
                .get("tool")
                .and_then(|v| v.as_str())
                .filter(|t| is_valid_tool(t))
                .filter(|t| {
                    // 审计#1：工具必须在指派 Agent 的白名单内
                    agents
                        .iter()
                        .find(|a| a.code == agent_code)
                        .and_then(|a| a.tools.as_ref())
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().any(|x| x.as_str() == Some(*t)))
                        .unwrap_or(false)
                })
                .map(|s| s.to_string());
            out.push(PlannedSubtask {
                title: it
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("未命名任务")
                    .to_string(),
                agent_code,
                description: it
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                tool_hint: tool,
                args: it.get("args").cloned().unwrap_or(json!({})),
                requires_approval: it
                    .get("requires_approval")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
            });
        }
    }
    if out.is_empty() {
        return Err(AppError::internal("CEO 拆解结果为空"));
    }
    Ok((out, bu))
}

pub async fn create_goal(
    state: &AppState,
    goal: &str,
    bu_code: Option<String>,
    created_by: &str,
) -> Result<Value, AppError> {
    let goal = goal.trim().to_string();
    if goal.is_empty() {
        return Err(AppError::bad_request("目标不能为空"));
    }
    let agents: Vec<Agent> = sqlx::query_as(
        "SELECT id,code,name,role,responsibilities,system_prompt,model,tools,max_depth,max_tool_calls,max_tokens,max_seconds,status FROM agents WHERE status='ACTIVE'",
    )
    .fetch_all(state.pool())
    .await?;
    let bu_codes: Vec<String> =
        sqlx::query_as("SELECT code FROM business_units WHERE status='ACTIVE' ORDER BY code")
            .fetch_all(state.pool())
            .await?
            .into_iter()
            .map(|(c,): (String,)| c)
            .collect();

    // 计划：AI 优先，失败自动落到离线模板（保证系统永远可用）
    let (plan_mode, subtasks, bu) = match ai_plan(state, &goal, &bu_codes, &agents).await {
        Ok((subs, bu)) => ("ai".to_string(), subs, bu.or_else(|| bu_code.clone())),
        Err(e) => {
            tracing::warn!("AI 拆解失败，使用离线模板: {e}");
            let bu = bu_code
                .clone()
                .unwrap_or_else(|| bu_codes.first().cloned().unwrap_or_else(|| "ERP".into()));
            let website: Option<(Option<String>,)> =
                sqlx::query_as("SELECT website FROM business_units WHERE code=$1")
                    .bind(&bu)
                    .fetch_optional(state.pool())
                    .await?;
            (
                "offline_fallback".to_string(),
                fallback_plan(
                    &goal,
                    &bu,
                    website.and_then(|(w,)| w).unwrap_or_default().trim(),
                ),
                Some(bu),
            )
        }
    };
    let bu = bu.unwrap_or_else(|| bu_code.clone().unwrap_or_else(|| "ERP".into()));

    let root_id = new_id();
    let plan_json = json!({
        "mode": plan_mode,
        "goal": goal,
        "subtasks": subtasks.iter().map(|s| json!({"title": s.title, "agent_code": s.agent_code, "tool": s.tool_hint})).collect::<Vec<_>>(),
    });
    sqlx::query("INSERT INTO tasks (id,title,goal,business_unit_code,agent_code,status,depth,plan,created_by) VALUES ($1,$2,$3,$4,'chief-of-staff','Planning',0,$5,$6)")
        .bind(&root_id)
        .bind(truncate(&goal, 80))
        .bind(&goal)
        .bind(&bu)
        .bind(plan_json)
        .bind(created_by)
        .execute(state.pool())
        .await?;

    let mut child_ids = Vec::new();
    for sub in &subtasks {
        let agent = agents.iter().find(|a| a.code == sub.agent_code);
        let id = new_id();
        sqlx::query(
            "INSERT INTO tasks (id,parent_id,root_id,title,description,goal,business_unit_code,agent_code,status,depth,tool_hint,args,requires_approval,max_tool_calls,max_tokens,max_seconds,created_by) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,'Created',1,$9,$10,$11,$12,$13,$14,$15)",
        )
        .bind(&id)
        .bind(&root_id)
        .bind(&root_id)
        .bind(truncate(&sub.title, 200))
        .bind(&sub.description)
        .bind(&goal)
        .bind(&bu)
        .bind(&sub.agent_code)
        .bind(sub.tool_hint.as_deref())
        .bind(&sub.args)
        .bind(sub.requires_approval)
        .bind(agent.map(|a| a.max_tool_calls).unwrap_or(12))
        .bind(agent.map(|a| a.max_tokens).unwrap_or(8000))
        .bind(agent.map(|a| a.max_seconds).unwrap_or(120))
        .bind(created_by)
        .execute(state.pool())
        .await?;
        add_event(
            state,
            &id,
            "Created",
            json!({"agent": sub.agent_code, "tool": sub.tool_hint}),
        )
        .await;
        child_ids.push(json!({"id": id, "title": sub.title, "agent_code": sub.agent_code, "tool_hint": sub.tool_hint, "requires_approval": sub.requires_approval}));
    }
    set_task_status(state, &root_id, "Assigned").await;
    add_event(
        state,
        &root_id,
        "Planned",
        json!({"mode": plan_mode, "children": child_ids.len()}),
    )
    .await;
    Ok(
        json!({"root_id": root_id, "mode": plan_mode, "business_unit_code": bu, "children": child_ids}),
    )
}

// ---------- 执行（状态机 + 预算） ----------

async fn run_agent_ai(state: &AppState, task: &Task) -> Result<Value, AppError> {
    if !state.0.cfg.ai_available() {
        return Err(AppError::not_configured(
            "AI 模型网关 (AI_BASE_URL/AI_API_KEY/AI_MODEL)",
        ));
    }
    let agent: Option<(Option<String>,)> =
        sqlx::query_as("SELECT system_prompt FROM agents WHERE code=$1")
            .bind(task.agent_code.as_deref().unwrap_or("ceo"))
            .fetch_optional(state.pool())
            .await?;
    let system = agent
        .and_then(|(s,)| s)
        .unwrap_or_else(|| "你是企业虚拟员工，直接输出可使用的工作成果。".into());
    let mems: Vec<(String, String)> = sqlx::query_as(
        "SELECT title, left(content,200) FROM memories WHERE status='ACTIVE' AND kind IN ('ENTERPRISE','KNOWLEDGE') ORDER BY created_at DESC LIMIT 5",
    )
    .fetch_all(state.pool())
    .await?;
    let mem_ctx = if mems.is_empty() {
        "（暂无相关记忆）".to_string()
    } else {
        mems.iter()
            .map(|(t, c)| format!("- {t}：{c}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let user = format!(
        "经营目标：{}\n\n你的任务：{}\n任务说明：{}\n\n企业记忆（供参考）：\n{mem_ctx}\n\n要求：直接输出工作成果，结尾一行以【结论】开头给出关键结论。",
        task.goal.as_deref().unwrap_or(""),
        task.title,
        task.description.as_deref().unwrap_or(""),
    );
    let provider = AiProvider {
        base_url: &state.0.cfg.ai_base_url,
        api_key: &state.0.cfg.ai_api_key,
        model: &state.0.cfg.ai_model,
        http: &state.0.http,
    };
    let reply = provider
        .chat(
            &[
                ChatMessage {
                    role: "system".into(),
                    content: system,
                },
                ChatMessage {
                    role: "user".into(),
                    content: user,
                },
            ],
            task.max_tokens.max(1) as u64,
        )
        .await?;
    Ok(json!({"output": reply.text, "tokens": reply.tokens, "mode": "ai"}))
}

/// 执行单个任务（带预算与超时；APPROVAL_REQUIRED 自动转审批）
pub async fn execute_task(state: &AppState, task_id: &str, approved: bool) -> Result<(), AppError> {
    let task: Task = sqlx::query_as("SELECT * FROM tasks WHERE id=$1")
        .bind(task_id)
        .fetch_optional(state.pool())
        .await?
        .ok_or_else(|| AppError::not_found("任务不存在"))?;
    if TERMINAL.contains(&task.status.as_str()) || task.status == "Running" {
        return Ok(());
    }
    // 审计#1/#2：预算护栏前置拦截（不再只记账）+ Agent 工具白名单
    if task.tool_calls_used >= task.max_tool_calls {
        return finish_failed(
            state,
            &task,
            AppError::forbidden("BUDGET_EXCEEDED: 工具调用次数已达上限"),
        )
        .await;
    }
    if task.tokens_used >= task.max_tokens {
        return finish_failed(
            state,
            &task,
            AppError::forbidden("BUDGET_EXCEEDED: Token 消耗已达预算上限"),
        )
        .await;
    }
    if let (Some(tool), Some(agent_code)) = (task.tool_hint.as_deref(), task.agent_code.as_deref())
    {
        let allowed: Option<(Option<serde_json::Value>,)> =
            sqlx::query_as("SELECT tools FROM agents WHERE code=$1")
                .bind(agent_code)
                .fetch_optional(state.pool())
                .await?;
        let in_whitelist = allowed
            .and_then(|(t,)| t)
            .and_then(|v| v.as_array().cloned())
            .map(|arr| arr.iter().any(|x| x.as_str() == Some(tool)))
            .unwrap_or(false);
        if !in_whitelist {
            return finish_failed(
                state,
                &task,
                AppError::forbidden(format!(
                    "Agent {agent_code} 无权调用工具 {tool}（超出白名单）"
                )),
            )
            .await;
        }
    }
    // 二审P2-4：原子状态抢占，杜绝并发双执行（先读后写竞态）
    let claim = sqlx::query("UPDATE tasks SET status='Running', started_at=now() WHERE id=$1 AND status IN ('Created','Planning','Assigned') RETURNING id")
        .bind(task_id)
        .fetch_optional(state.pool())
        .await?;
    if claim.is_none() {
        return Ok(()); // 已被其他执行者抢占或已终态
    }
    add_event(state, task_id, "Running", json!({"approved": approved})).await;

    let tool_hint = task.tool_hint.clone();
    let args = task.args.clone().unwrap_or(json!({}));
    let secs = task.max_seconds.max(1) as u64;
    let state2 = state.clone();
    let task_id2 = task_id.to_string();
    let approved2 = approved;
    let fut_tool = tool_hint.clone();
    let fut_args = args.clone();
    let fut_task = task.clone();
    let fut = async move {
        if let Some(tool) = &fut_tool {
            let ctx = ToolCtx {
                state: &state2,
                task_id: task_id2.clone(),
                approved: approved2,
            };
            dispatch(tool, fut_args, &ctx).await
        } else {
            run_agent_ai(&state2, &fut_task).await
        }
    };
    let result = match tokio::time::timeout(Duration::from_secs(secs), fut).await {
        Ok(r) => r,
        Err(_) => Err(AppError::new(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            "TIMEOUT",
            format!("任务执行超过 {secs}s 预算上限，已中止"),
        )),
    };

    match result {
        Ok(value) => {
            let tokens = value.get("tokens").and_then(|v| v.as_u64()).unwrap_or(0) as i32;
            let used_tool = tool_hint.is_some();
            // 三审P1-C：完成守卫——取消过的任务不被覆盖成 Completed；外部副作用结果照实入档
            let cancelled_midway: Option<(String,)> =
                sqlx::query_as("SELECT status FROM tasks WHERE id=$1")
                    .bind(task_id)
                    .fetch_optional(state.pool())
                    .await?
                    .filter(|(s,)| s == "CancelRequested");
            let final_status = if cancelled_midway.is_some() {
                "Cancelled"
            } else {
                "Completed"
            };
            let mut value = value;
            if let Some(obj) = value.as_object_mut() {
                obj.insert("cancelled_midway".into(), json!(cancelled_midway.is_some()));
            }
            let archive_note = if final_status == "Cancelled" {
                Some("外部副作用已执行，结果照实存档（取消请求在执行中到达）".to_string())
            } else {
                None
            };
            sqlx::query("UPDATE tasks SET status=$2, finished_at=now(), result=$3, tool_calls_used=tool_calls_used+$4, tokens_used=tokens_used+$5, error=COALESCE($6, error) WHERE id=$1 AND status='Running'")
                .bind(task_id)
                .bind(final_status)
                .bind(&value)
                .bind(if used_tool { 1 } else { 0 })
                .bind(tokens)
                .bind(archive_note)
                .execute(state.pool())
                .await?;
            add_event(
                state,
                task_id,
                if final_status == "Cancelled" {
                    "CancelledAfterEffect"
                } else {
                    "Completed"
                },
                json!({"mode": value.get("mode")}),
            )
            .await;
            // Memory Manager 钩子：任务记忆自动沉淀（TASK 类直接生效）
            let summary = value
                .get("output")
                .and_then(|v| v.as_str())
                .map(|s| truncate(s, 500))
                .unwrap_or_default();
            if !summary.is_empty() {
                let _ = sqlx::query("INSERT INTO memories (id,kind,title,content,source_task_id,status) VALUES ($1,'TASK',$2,$3,$4,'ACTIVE')")
                    .bind(new_id())
                    .bind(format!("【任务记忆】{}", task.title))
                    .bind(summary)
                    .bind(task_id)
                    .execute(state.pool())
                    .await;
            }
        }
        Err(e) if e.message.starts_with("APPROVAL_REQUIRED") => {
            let existing: Option<(String,)> =
                sqlx::query_as("SELECT id FROM approvals WHERE task_id=$1 AND status='PENDING'")
                    .bind(task_id)
                    .fetch_optional(state.pool())
                    .await?;
            if existing.is_none() {
                create_approval(state, &task, "system").await?;
            }
            set_task_status(state, task_id, "Waiting Approval").await;
            add_event(
                state,
                task_id,
                "WaitingApproval",
                json!({"reason": e.message}),
            )
            .await;
        }
        Err(e) => {
            sqlx::query(
                "UPDATE tasks SET status='Failed', finished_at=now(), error=$2 WHERE id=$1",
            )
            .bind(task_id)
            .bind(format!("[{}] {}", e.code, e.message))
            .execute(state.pool())
            .await?;
            add_event(state, task_id, "Failed", json!({"error": e.message})).await;
        }
    }
    Ok(())
}

/// 执行整个目标：跑所有子任务（需审批的挂审批），最后汇总根任务状态
pub async fn execute_goal(state: &AppState, root_id: &str) -> Result<Value, AppError> {
    let children: Vec<Task> =
        sqlx::query_as("SELECT * FROM tasks WHERE root_id=$1 AND depth=1 ORDER BY created_at")
            .bind(root_id)
            .fetch_all(state.pool())
            .await?;
    for child in &children {
        match child.status.as_str() {
            "Created" if child.requires_approval => {
                let approved: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM approvals WHERE task_id=$1 AND status='APPROVED'",
                )
                .bind(&child.id)
                .fetch_optional(state.pool())
                .await?;
                if approved.is_some() {
                    execute_task(state, &child.id, true).await?;
                } else {
                    let pending: Option<(String,)> = sqlx::query_as(
                        "SELECT id FROM approvals WHERE task_id=$1 AND status='PENDING'",
                    )
                    .bind(&child.id)
                    .fetch_optional(state.pool())
                    .await?;
                    if pending.is_none() {
                        create_approval(state, child, "system").await?;
                        set_task_status(state, &child.id, "Waiting Approval").await;
                        add_event(
                            state,
                            &child.id,
                            "WaitingApproval",
                            json!({"reason": "requires_approval"}),
                        )
                        .await;
                    }
                }
            }
            "Created" | "Waiting Approval" => {}
            _ => continue,
        }
    }
    // 逐个执行可运行任务
    let children2: Vec<Task> =
        sqlx::query_as("SELECT * FROM tasks WHERE root_id=$1 AND depth=1 ORDER BY created_at")
            .bind(root_id)
            .fetch_all(state.pool())
            .await?;
    for child in &children2 {
        if child.status == "Created" {
            execute_task(state, &child.id, false).await?;
        }
    }
    // 汇总（sync_root_status 统一判定，含 PartialFailed 语义）
    sync_root_status(state, root_id).await
}

/// 审批决定：批准→带授权执行；拒绝→任务取消
pub async fn decide_approval(
    state: &AppState,
    approval_id: &str,
    decision: &str,
    decided_by: &str,
    decided_by_role: &str,
    note: &str,
) -> Result<Value, AppError> {
    if !["APPROVED", "REJECTED"].contains(&decision) {
        return Err(AppError::bad_request(
            "decision 必须为 APPROVED 或 REJECTED",
        ));
    }
    // 三审P1：事务 + FOR UPDATE 行锁 + PENDING 条件更新——并发审批只能成功一次
    let mut tx = state.pool().begin().await?;
    let approval: Option<(String, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT task_id, status, requested_by, args_hash FROM approvals WHERE id=$1 FOR UPDATE",
    )
    .bind(approval_id)
    .fetch_optional(&mut *tx)
    .await?;
    let (task_id, status, requested_by, expected_hash) =
        approval.ok_or_else(|| AppError::not_found("审批不存在"))?;
    if status != "PENDING" {
        return Err(AppError::conflict(format!("该审批已处理（{status}）")));
    }
    // 三审明确规则：任务创建者本人不得自批（ADMIN 例外；system 发起的由审批员批）
    if let Some(req) = &requested_by {
        if req == decided_by && req != "system" && decided_by_role != "ADMIN" {
            return Err(AppError::forbidden(
                "任务创建者本人不能审批该任务（利益冲突）",
            ));
        }
    }
    let upd = sqlx::query("UPDATE approvals SET status=$2, decided_by=$3, decided_at=now(), note=$4 WHERE id=$1 AND status='PENDING'")
        .bind(approval_id)
        .bind(decision)
        .bind(decided_by)
        .bind(note)
        .execute(&mut *tx)
        .await?;
    if upd.rows_affected() == 0 {
        return Err(AppError::conflict("该审批已被并发处理"));
    }
    // 三审P1：快照校验在事务内完成——不一致即整体回滚，绝不留下 APPROVED
    if decision == "APPROVED" {
        if let Some(expected) = &expected_hash {
            let current_task: Task = sqlx::query_as("SELECT * FROM tasks WHERE id=$1")
                .bind(&task_id)
                .fetch_one(&mut *tx)
                .await?;
            let current = args_hash(&current_task.args.clone().unwrap_or(json!({})));
            if *expected != current {
                return Err(AppError::conflict(
                    "审批后的任务参数已发生变化，必须重新提交审批",
                ));
            }
        }
    }
    // 三审T-002：同一事务内把任务置为 Assigned（可被 execute_task 原子抢单认领）
    if decision == "APPROVED" {
        sqlx::query("UPDATE tasks SET status='Assigned' WHERE id=$1")
            .bind(&task_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    // 事务提交后按决定执行副作用
    match decision {
        "APPROVED" => {
            add_event(
                state,
                &task_id,
                "ApprovalGranted",
                json!({"by": decided_by}),
            )
            .await;
            execute_task(state, &task_id, true).await?;
        }
        _ => {
            set_task_status(state, &task_id, "Rejected").await;
            add_event(
                state,
                &task_id,
                "ApprovalRejected",
                json!({"by": decided_by, "note": note}),
            )
            .await;
        }
    }
    let task: Task = sqlx::query_as("SELECT * FROM tasks WHERE id=$1")
        .bind(&task_id)
        .fetch_one(state.pool())
        .await?;
    let root_summary = match &task.root_id {
        Some(root) => Some(sync_root_status(state, root).await?),
        None => None,
    };
    Ok(
        json!({"task_id": task.id, "status": task.status, "result": task.result, "root": root_summary}),
    )
}

/// 统一失败落账（供预算/白名单前置拦截复用）
async fn finish_failed(state: &AppState, task: &Task, e: AppError) -> Result<(), AppError> {
    sqlx::query("UPDATE tasks SET status='Failed', finished_at=now(), error=$2 WHERE id=$1")
        .bind(&task.id)
        .bind(format!("[{}] {}", e.code, e.message))
        .execute(state.pool())
        .await?;
    add_event(state, &task.id, "Failed", json!({"error": e.message})).await;
    Ok(())
}

/// 审计#6：统一汇总根任务状态（execute_goal / decide_approval / retry / cancel 后都必须调用）
pub async fn sync_root_status(state: &AppState, root_id: &str) -> Result<Value, AppError> {
    let children: Vec<Task> =
        sqlx::query_as("SELECT * FROM tasks WHERE root_id=$1 AND depth=1 ORDER BY created_at")
            .bind(root_id)
            .fetch_all(state.pool())
            .await?;
    if children.is_empty() {
        return Ok(json!({}));
    }
    let mut completed = 0i64;
    let mut failed = 0i64;
    let mut waiting = 0i64;
    let mut active = 0i64;
    for c in &children {
        match c.status.as_str() {
            "Completed" => completed += 1,
            "Failed" | "Cancelled" | "Rejected" => failed += 1,
            "Waiting Approval" => waiting += 1,
            _ => active += 1,
        }
    }
    // 不再说谎：全败=Failed，有等待=Waiting Approval，有活跃=Running，其余=Completed
    let root_status = if active > 0 {
        "Running"
    } else if waiting > 0 {
        "Waiting Approval"
    } else if failed > 0 && completed > 0 {
        "PartialFailed" // 二审P1-1：部分失败不再谎报 Completed
    } else if failed > 0 {
        "Failed"
    } else {
        "Completed"
    };
    let is_terminal = matches!(root_status, "Completed" | "Failed");
    let summary = json!({"completed": completed, "failed": failed, "waiting_approval": waiting});
    sqlx::query("UPDATE tasks SET status=$2, finished_at=CASE WHEN $3 THEN now() ELSE finished_at END, result=$4 WHERE id=$1")
        .bind(root_id)
        .bind(root_status)
        .bind(is_terminal)
        .bind(&summary)
        .execute(state.pool())
        .await?;
    add_event(state, root_id, "GoalSummary", summary.clone()).await;
    Ok(summary)
}
