use crate::error::AppError;
use serde_json::{json, Value};

/// 统一模型网关适配器（OpenAI 兼容 /chat/completions 协议）。
/// 可指向 one-api、GLM、OpenAI、Claude 兼容网关等任意供应商。
pub struct AiProvider<'a> {
    pub base_url: &'a str,
    pub api_key: &'a str,
    pub model: &'a str,
    pub http: &'a reqwest::Client,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

pub struct ChatReply {
    pub text: String,
    pub tokens: u64,
    pub model: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub estimated: bool,
}

impl<'a> AiProvider<'a> {
    pub async fn chat(
        &self,
        messages: &[ChatMessage],
        max_tokens: u64,
    ) -> Result<ChatReply, AppError> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let body = json!({
            "model": self.model,
            "messages": messages,
            "max_tokens": max_tokens,
            "temperature": 0.4,
        });
        let resp = self
            .http
            .post(&url)
            .bearer_auth(self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("AI 网关连接失败: {e}")))?;
        let status = resp.status();
        let payload: Value = resp
            .json()
            .await
            .map_err(|e| AppError::internal(format!("AI 网关返回解析失败: {e}")))?;
        if !status.is_success() {
            return Err(AppError::internal(format!(
                "AI 网关返回 {status}: {}",
                payload
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("未知错误")
            )));
        }
        let text = payload["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        // T-006：网关未返回 usage 时按字符数估算并标记 estimated
        let prompt_tokens = payload["usage"]["prompt_tokens"].as_u64();
        let completion_tokens = payload["usage"]["completion_tokens"].as_u64();
        let (prompt_tokens, completion_tokens, estimated) = match (prompt_tokens, completion_tokens) {
            (Some(p), Some(c)) => (p, c, false),
            _ => ((messages.iter().map(|m| m.content.chars().count()).sum::<usize>() as u64) / 2 + 1, (text.chars().count() as u64) / 2 + 1, true),
        };
        let tokens = payload["usage"]["total_tokens"].as_u64().unwrap_or(prompt_tokens + completion_tokens);
        Ok(ChatReply { text, tokens, model: self.model.to_string(), prompt_tokens, completion_tokens, estimated })
    }
}

/// 从模型输出中提取 JSON（容忍 ```json 包裹）
pub fn extract_json(text: &str) -> Option<Value> {
    // 审计#14：优先 ```json 围栏，避免外层散文大括号污染
    if let Some(pos) = text.find("```json") {
        let rest = &text[pos + 7..];
        if let Some(end) = rest.find("```") {
            if let Ok(v) = serde_json::from_str(rest[..end].trim()) {
                return Some(v);
            }
        }
    }
    let trimmed = text.trim();
    let candidate = if let Some(start) = trimmed.find('{') {
        let end = trimmed.rfind('}')?;
        &trimmed[start..=end]
    } else {
        trimmed
    };
    serde_json::from_str(candidate).ok()
}

/// Embedding 客户端（OpenAI 兼容 /embeddings 协议）
pub struct EmbedClient<'a> {
    pub base_url: &'a str,
    pub api_key: &'a str,
    pub model: &'a str,
    pub http: &'a reqwest::Client,
}

impl<'a> EmbedClient<'a> {
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>, AppError> {
        let url = format!("{}/embeddings", self.base_url.trim_end_matches('/'));
        let resp = self
            .http
            .post(&url)
            .bearer_auth(self.api_key)
            .json(&json!({ "model": self.model, "input": [text] }))
            .send()
            .await
            .map_err(|e| AppError::internal(format!("Embedding 服务连接失败: {e}")))?;
        let status = resp.status();
        let payload: Value = resp
            .json()
            .await
            .map_err(|e| AppError::internal(format!("Embedding 返回解析失败: {e}")))?;
        if !status.is_success() {
            return Err(AppError::internal(format!("Embedding 服务返回 {status}")));
        }
        let vec: Vec<f32> = payload["data"][0]["embedding"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                    .collect()
            })
            .unwrap_or_default();
        if vec.is_empty() {
            return Err(AppError::internal("Embedding 返回为空"));
        }
        Ok(vec)
    }
}

/// 余弦相似度
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0f32;
    let mut na = 0f32;
    let mut nb = 0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na.sqrt() * nb.sqrt())
    }
}

/// 把向量序列化为 pgvector 文本格式
pub fn vector_to_pgtext(v: &[f32]) -> String {
    let items: Vec<String> = v.iter().map(|f| format!("{f}")).collect();
    format!("[{}]", items.join(","))
}

pub fn parse_pgvector(s: &str) -> Vec<f32> {
    s.trim_matches(|c| c == '[' || c == ']')
        .split(',')
        .filter_map(|x| x.trim().parse::<f32>().ok())
        .collect()
}


/// T-006：所有 AI 调用的统一入口——日预算闸门（配置时区当日）+ 调用 + 台账（失败显式记日志）
pub async fn gated_chat(
    state: &crate::state::AppState,
    task_id: Option<&str>,
    system: &str,
    user: &str,
    max_tokens: u64,
) -> Result<ChatReply, crate::error::AppError> {
    let tz = &state.0.cfg.ai_timezone;
    let gate = |what: &str| {
        crate::error::AppError::new(
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            "BUDGET_EXCEEDED",
            format!("当日{}预算已用尽", what),
        )
    };
    if !state.0.cfg.ai_available() {
        return Err(crate::error::AppError::not_configured(
            "AI 模型网关 (AI_BASE_URL/AI_API_KEY/AI_MODEL)",
        ));
    }
    let (used_tokens, used_cost): (i64, f64) = sqlx::query_as(
        "SELECT COALESCE(SUM(tokens),0)::bigint, COALESCE(SUM(cost_usd),0)::float8 FROM usage_ledger WHERE (created_at AT TIME ZONE $1)::date = (now() AT TIME ZONE $1)::date",
    )
    .bind(tz)
    .fetch_one(state.pool())
    .await
    .unwrap_or((0, 0.0)); // 查询失败不阻断，record_usage 侧会显式记日志
    if state.0.cfg.daily_token_limit > 0 && used_tokens >= state.0.cfg.daily_token_limit {
        return Err(gate(&format!("Token（{used_tokens}/{}）", state.0.cfg.daily_token_limit)));
    }
    if state.0.cfg.daily_cost_limit_usd > 0.0 && used_cost >= state.0.cfg.daily_cost_limit_usd {
        return Err(gate(&format!("费用（${used_cost:.2}/${:.2}）", state.0.cfg.daily_cost_limit_usd)));
    }
    let provider = AiProvider {
        base_url: &state.0.cfg.ai_base_url,
        api_key: &state.0.cfg.ai_api_key,
        model: &state.0.cfg.ai_model,
        http: &state.0.http,
    };
    let reply = provider
        .chat(
            &[ChatMessage { role: "system".into(), content: system.into() }, ChatMessage { role: "user".into(), content: user.into() }],
            max_tokens,
        )
        .await?;
    record_usage(state, task_id, &reply).await;
    Ok(reply)
}

/// T-006：台账写入——按模型计价（AI_PRICING_JSON 子串匹配，回退统一费率）；失败显式 log 不静默
pub async fn record_usage(
    state: &crate::state::AppState,
    task_id: Option<&str>,
    reply: &ChatReply,
) {
    if reply.tokens == 0 && reply.estimated {
        return;
    }
    let rate = serde_json::from_str::<Value>(&state.0.cfg.ai_pricing_json)
        .ok()
        .and_then(|v| {
            let model = reply.model.to_lowercase();
            v.as_object()
                .and_then(|o| {
                    o.iter()
                        .find(|(k, _)| model.contains(&k.to_lowercase()))
                        .and_then(|(_, v)| v.as_f64())
                })
        })
        .unwrap_or(state.0.cfg.ai_cost_per_1k_usd);
    let cost = reply.tokens as f64 / 1000.0 * rate;
    if let Err(e) = sqlx::query(
        "INSERT INTO usage_ledger (id, task_id, tokens, cost_usd, model, prompt_tokens, completion_tokens, estimated) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(crate::auth::new_id())
    .bind(task_id)
    .bind(reply.tokens as i64)
    .bind(cost)
    .bind(&reply.model)
    .bind(reply.prompt_tokens as i64)
    .bind(reply.completion_tokens as i64)
    .bind(reply.estimated)
    .execute(state.pool())
    .await
    {
        tracing::error!("usage_ledger 写入失败（{}tokens, model={}）: {e}", reply.tokens, reply.model);
    }
}
