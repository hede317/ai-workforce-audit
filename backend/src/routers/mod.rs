mod agents;
mod business;
mod core;
mod data;

use crate::auth::new_id;
use crate::state::AppState;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::Router;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        // 认证
        .route("/api/v1/auth/login", post(core::login))
        .route("/api/v1/auth/me", get(core::me))
        // 业务单元
        .route("/api/v1/business-units", get(core::list_bus).post(core::create_bu))
        // 审计
        .route("/api/v1/audit", get(core::list_audit))
        // Agent 中心
        .route("/api/v1/agents", get(agents::list_agents))
        .route("/api/v1/agents/:id", put(agents::update_agent))
        .route("/api/v1/tools", get(agents::list_tools))
        .route("/api/v1/tools/:code/invoke", post(agents::invoke_tool))
        // AI Commander
        .route("/api/v1/commander/goals", post(agents::create_goal))
        .route("/api/v1/commander/goals/:id", get(agents::get_goal))
        .route("/api/v1/commander/goals/:id/execute", post(agents::execute_goal))
        // 任务
        .route("/api/v1/tasks", get(agents::list_tasks))
        .route("/api/v1/tasks/:id", get(agents::get_task))
        .route("/api/v1/tasks/:id/retry", post(agents::retry_task))
        .route("/api/v1/tasks/:id/cancel", post(agents::cancel_task))
        // 审批
        .route("/api/v1/approvals", get(agents::list_approvals))
        .route("/api/v1/approvals/:id/decide", post(agents::decide_approval))
        // 内容
        .route("/api/v1/topics", get(business::list_topics).post(business::create_topic))
        .route("/api/v1/contents", get(business::list_contents).post(business::create_content))
        .route("/api/v1/contents/:id", get(business::get_content))
        .route("/api/v1/contents/:id/approve", post(business::approve_content))
        .route("/api/v1/contents/:id/publish", post(business::publish_content))
        // 线索
        .route("/api/v1/leads", get(business::list_leads).post(business::create_lead))
        .route("/api/v1/leads/:id/events", get(business::list_lead_events).post(business::add_lead_event))
        // 活动
        .route("/api/v1/campaigns", get(business::list_campaigns).post(business::create_campaign))
        // 发布中心
        .route("/api/v1/products", get(business::list_products).post(business::create_product))
        .route("/api/v1/products/:id", put(business::update_product))
        .route("/api/v1/publishing", get(business::list_publishing))
        .route("/api/v1/publishing/:id/result", post(business::record_publishing_result))
        // 记忆
        .route("/api/v1/memories", get(data::list_memories).post(data::create_memory))
        .route("/api/v1/memories/:id", put(data::update_memory))
        // 分析与工作流
        .route("/api/v1/analytics/summary", get(data::analytics_summary))
        .route("/api/v1/analytics/daily-report", get(data::daily_report))
        .route("/api/v1/workflows/trigger", post(data::trigger_workflow))
        .route("/api/v1/workflows/executions", get(data::list_executions))
        .layer(axum::middleware::from_fn_with_state(state.clone(), idempotency_mw))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

/// 幂等中间件：带 Idempotency-Key 的 POST 不会重复产生副作用（规范第十部分）
async fn idempotency_mw(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let key = req
        .headers()
        .get("Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let is_post = req.method() == axum::http::Method::POST;
    let endpoint = req.uri().path().to_string();
    match (is_post, key) {
        (true, Some(key)) => {
            let existing: Option<(serde_json::Value,)> =
                sqlx::query_as("SELECT response FROM idempotency_keys WHERE idem_key=$1 AND endpoint=$2")
                    .bind(&key)
                    .bind(&endpoint)
                    .fetch_optional(state.pool())
                    .await
                    .ok()
                    .flatten();
            if let Some((body,)) = existing {
                return axum::Json(body).into_response();
            }
            let res = next.run(req).await;
            let (parts, body) = res.into_parts();
            let bytes = axum::body::to_bytes(body, 2_000_000).await.unwrap_or_default();
            if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                let _ = sqlx::query(
                    "INSERT INTO idempotency_keys (id, idem_key, endpoint, response) VALUES ($1,$2,$3,$4) ON CONFLICT (idem_key) DO NOTHING",
                )
                .bind(new_id())
                .bind(&key)
                .bind(&endpoint)
                .bind(&value)
                .execute(state.pool())
                .await;
            }
            Response::from_parts(parts, Body::from(bytes))
        }
        _ => next.run(req).await,
    }
}
