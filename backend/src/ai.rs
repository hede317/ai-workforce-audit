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
}

impl<'a> AiProvider<'a> {
    pub async fn chat(&self, messages: &[ChatMessage], max_tokens: u64) -> Result<ChatReply, AppError> {
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
                payload.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()).unwrap_or("未知错误")
            )));
        }
        let text = payload["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let tokens = payload["usage"]["total_tokens"].as_u64().unwrap_or(0);
        Ok(ChatReply { text, tokens })
    }
}

/// 从模型输出中提取 JSON（容忍 ```json 包裹）
pub fn extract_json(text: &str) -> Option<Value> {
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
        let payload: Value = resp.json().await.map_err(|e| AppError::internal(format!("Embedding 返回解析失败: {e}")))?;
        if !status.is_success() {
            return Err(AppError::internal(format!("Embedding 服务返回 {status}")));
        }
        let vec: Vec<f32> = payload["data"][0]["embedding"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect())
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
