mod agents;
mod botarmy;
mod business;
mod core;
mod data;
mod listening;

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
        .route(
            "/api/v1/business-units",
            get(core::list_bus).post(core::create_bu),
        )
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
        .route(
            "/api/v1/commander/goals/:id/execute",
            post(agents::execute_goal),
        )
        // 任务
        .route("/api/v1/tasks", get(agents::list_tasks))
        .route("/api/v1/tasks/:id", get(agents::get_task))
        .route("/api/v1/tasks/:id/retry", post(agents::retry_task))
        .route("/api/v1/tasks/:id/cancel", post(agents::cancel_task))
        // 审批
        .route("/api/v1/approvals", get(agents::list_approvals))
        .route(
            "/api/v1/approvals/:id/decide",
            post(agents::decide_approval),
        )
        // 内容
        .route(
            "/api/v1/topics",
            get(business::list_topics).post(business::create_topic),
        )
        .route(
            "/api/v1/contents",
            get(business::list_contents).post(business::create_content),
        )
        .route("/api/v1/contents/:id", get(business::get_content))
        .route(
            "/api/v1/contents/:id/approve",
            post(business::approve_content),
        )
        .route(
            "/api/v1/contents/:id/publish",
            post(business::publish_content),
        )
        // 线索
        .route(
            "/api/v1/leads",
            get(business::list_leads).post(business::create_lead),
        )
        // 舆情雷达
        .route("/api/v1/botarmy", get(botarmy::list_bots))
        .route(
            "/api/v1/listening/keywords",
            get(listening::list_keywords).post(listening::add_keyword),
        )
        .route(
            "/api/v1/listening/keywords/:id",
            axum::routing::delete(listening::delete_keyword),
        )
        .route("/api/v1/listening/signals", get(listening::list_signals))
        .route(
            "/api/v1/listening/signals/:id/to-topic",
            post(listening::signal_to_topic),
        )
        .route(
            "/api/v1/listening/signals/:id/to-lead",
            post(listening::signal_to_lead),
        )
        .route(
            "/api/v1/leads/:id/events",
            get(business::list_lead_events).post(business::add_lead_event),
        )
        // 活动
        .route(
            "/api/v1/campaigns",
            get(business::list_campaigns).post(business::create_campaign),
        )
        // 发布中心
        .route(
            "/api/v1/products",
            get(business::list_products).post(business::create_product),
        )
        .route("/api/v1/products/:id", put(business::update_product))
        .route("/api/v1/publishing", get(business::list_publishing))
        .route(
            "/api/v1/publishing/:id/result",
            post(business::record_publishing_result),
        )
        // 记忆
        .route(
            "/api/v1/memories",
            get(data::list_memories).post(data::create_memory),
        )
        .route("/api/v1/memories/:id", put(data::update_memory))
        // 分析与工作流
        .route("/api/v1/usage/today", get(data::usage_today))
        .route("/api/v1/analytics/summary", get(data::analytics_summary))
        .route("/api/v1/analytics/daily-report", get(data::daily_report))
        .route("/api/v1/workflows/trigger", post(data::trigger_workflow))
        .route("/api/v1/workflows/executions", get(data::list_executions))
        .layer(axum::middleware::from_fn(request_id_mw))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            token_renewal_mw,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            idempotency_mw,
        ))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

/// 幂等中间件 v2（审计#4）：原子抢占防 TOCTOU 竞态；仅缓存 2xx；失败删除占位允许重试
async fn idempotency_mw(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let key = req
        .headers()
        .get("Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let is_post = req.method() == axum::http::Method::POST;
    let endpoint = req.uri().path().to_string();
    match (is_post, key) {
        (true, Some(key)) => {
            // 二审P1-2：先回收陈锁（进程崩溃残留的 NULL 占位），防幂等键永久锁死
            let _ = sqlx::query(
                "DELETE FROM idempotency_keys WHERE endpoint=$1 AND idem_key=$2 AND response IS NULL AND created_at < now() - interval '10 minutes'",
            )
            .bind(&endpoint)
            .bind(&key)
            .execute(state.pool())
            .await;
            // 1. 原子抢占占位（response 为 NULL = 执行中）
            let claim = sqlx::query(
                "INSERT INTO idempotency_keys (id, idem_key, endpoint, response) VALUES ($1,$2,$3,NULL) ON CONFLICT DO NOTHING",
            )
            .bind(new_id())
            .bind(&key)
            .bind(&endpoint)
            .execute(state.pool())
            .await;
            match claim {
                Ok(res) if res.rows_affected() == 0 => {
                    // 已被抢占：有结果回放结果，无结果=处理中
                    let existing: Option<(Option<serde_json::Value>,)> = sqlx::query_as(
                        "SELECT response FROM idempotency_keys WHERE endpoint=$1 AND idem_key=$2",
                    )
                    .bind(&endpoint)
                    .bind(&key)
                    .fetch_optional(state.pool())
                    .await
                    .ok()
                    .flatten();
                    return match existing {
                        Some((Some(body),)) => {
                            (axum::http::StatusCode::OK, axum::Json(body)).into_response()
                        }
                        _ => crate::error::AppError::conflict(
                            "相同幂等键的请求正在处理中，请稍后重试",
                        )
                        .into_response(),
                    };
                }
                Err(_) => return next.run(req).await, // 抢占本身失败时不拦截业务
                _ => {}
            }
            // 2. 执行业务
            let res = next.run(req).await;
            let (parts, body) = res.into_parts();
            let bytes = axum::body::to_bytes(body, 8_000_000)
                .await
                .unwrap_or_default();
            // 3. 仅 2xx 持久化；失败删除占位，客户端可携带同键重试
            if parts.status.is_success() {
                if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                    let _ = sqlx::query(
                        "UPDATE idempotency_keys SET response=$3 WHERE endpoint=$1 AND idem_key=$2",
                    )
                    .bind(&endpoint)
                    .bind(&key)
                    .bind(&value)
                    .execute(state.pool())
                    .await;
                }
            } else {
                let _ =
                    sqlx::query("DELETE FROM idempotency_keys WHERE endpoint=$1 AND idem_key=$2")
                        .bind(&endpoint)
                        .bind(&key)
                        .execute(state.pool())
                        .await;
            }
            Response::from_parts(parts, Body::from(bytes))
        }
        _ => next.run(req).await,
    }
}

/// 尾巴#3：每请求生成 Request-Id 并回写响应头，排障可关联
async fn request_id_mw(req: Request, next: Next) -> Response {
    let rid = axum::http::HeaderValue::from_str(&uuid::Uuid::new_v4().simple().to_string())
        .unwrap_or_else(|_| axum::http::HeaderValue::from_static("unknown"));
    let mut res = next.run(req).await;
    res.headers_mut().insert("X-Request-Id", rid);
    res
}

/// 令牌滑动续期（审计#3）：剩余 <6h 时自动签发新令牌放入响应头 X-Renewed-Token
async fn token_renewal_mw(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let auth = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(|s| s.to_string());
    let res = next.run(req).await;
    if let Some(token) = auth {
        if let Ok(claims) = crate::auth::decode_jwt(&state.0.cfg.jwt_secret, &token) {
            let remain = claims.exp - chrono::Utc::now().timestamp();
            if remain > 0 && remain < 6 * 3600 {
                if let Ok(new_tok) = crate::auth::create_jwt(
                    &state.0.cfg.jwt_secret,
                    &claims.sub,
                    &claims.username,
                    &claims.role,
                    &claims.pv,
                ) {
                    let mut res = res;
                    if let Ok(v) = axum::http::HeaderValue::from_str(&new_tok) {
                        res.headers_mut().insert("X-Renewed-Token", v);
                    }
                    return res;
                }
            }
        }
    }
    res
}
