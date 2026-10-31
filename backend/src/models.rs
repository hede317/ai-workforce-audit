use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, FromRow, Serialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct BusinessUnit {
    pub id: String,
    pub code: String,
    pub name: String,
    pub website: String,
    pub description: Option<String>,
    pub default_language: String,
    pub configuration: Option<serde_json::Value>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize, Clone)]
pub struct Agent {
    pub id: String,
    pub code: String,
    pub name: String,
    pub role: String,
    pub responsibilities: Option<String>,
    pub system_prompt: Option<String>,
    pub model: Option<String>,
    pub tools: Option<serde_json::Value>,
    pub max_depth: i32,
    pub max_tool_calls: i32,
    pub max_tokens: i32,
    pub max_seconds: i32,
    pub status: String,
}

#[derive(Debug, FromRow, Serialize, Clone)]
pub struct Task {
    pub id: String,
    pub parent_id: Option<String>,
    pub root_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub goal: Option<String>,
    pub business_unit_code: Option<String>,
    pub agent_code: Option<String>,
    pub status: String,
    pub priority: i32,
    pub depth: i32,
    pub tool_hint: Option<String>,
    pub args: Option<serde_json::Value>,
    pub plan: Option<serde_json::Value>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub requires_approval: bool,
    pub max_tool_calls: i32,
    pub max_tokens: i32,
    pub max_seconds: i32,
    pub tool_calls_used: i32,
    pub tokens_used: i32,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct Approval {
    pub id: String,
    pub task_id: String,
    pub action: String,
    pub status: String,
    pub requested_by: Option<String>,
    pub decided_by: Option<String>,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub decided_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct Memory {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub content: String,
    pub tags: Option<serde_json::Value>,
    pub source_task_id: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct ToolRow {
    pub id: String,
    pub code: String,
    pub name: String,
    pub category: String,
    pub description: Option<String>,
    pub input_schema: Option<serde_json::Value>,
    pub permission_level: String,
    pub timeout_seconds: i32,
    pub status: String,
}

#[derive(Debug, FromRow, Serialize)]
pub struct Lead {
    pub id: String,
    pub business_unit_code: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub country: Option<String>,
    pub source_channel: Option<String>,
    pub source_content_id: Option<String>,
    pub consent_status: String,
    pub lead_status: String,
    pub intent_level: Option<String>,
    pub summary: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct ContentTopic {
    pub id: String,
    pub business_unit_code: Option<String>,
    pub title: String,
    pub keywords: Option<serde_json::Value>,
    pub audience: Option<String>,
    pub market: Option<String>,
    pub language: String,
    pub priority: i32,
    pub status: String,
    pub source_task_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct PublishingTask {
    pub id: String,
    pub content_asset_id: Option<String>,
    pub platform: String,
    pub account: Option<String>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub status: String,
    pub payload: Option<serde_json::Value>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
}
