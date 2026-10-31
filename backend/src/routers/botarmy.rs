use crate::auth::AuthUser;
use crate::error::{ok, ApiResult};
use crate::state::AppState;
use axum::extract::State;
use serde_json::json;

/// DSH BOT 军团岗位清单（技能真源 ~/.agents/skills/army-*）
pub async fn list_bots(State(state): State<AppState>, _user: AuthUser) -> ApiResult {
    let _ = &state;
    let dir = std::path::Path::new(&std::env::var("HOME").unwrap_or_default()).join(".agents/skills");
    let mut items = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let fname = e.file_name().to_string_lossy().to_string();
            if !fname.starts_with("army-") {
                continue;
            }
            let skill = e.path().join("SKILL.md");
            if let Ok(content) = std::fs::read_to_string(&skill) {
                let fm = content.split("---").nth(1).unwrap_or("");
                let mut name = String::new();
                let mut desc = String::new();
                for line in fm.lines() {
                    if let Some(v) = line.strip_prefix("name:") {
                        name = v.trim().to_string();
                    } else if let Some(v) = line.strip_prefix("description:") {
                        desc = v.trim().to_string();
                    }
                }
                if !name.is_empty() {
                    items.push(json!({"bot": fname, "name": name, "description": desc.chars().take(140).collect::<String>()}));
                }
            }
        }
    }
    items.sort_by(|a, b| a["bot"].as_str().unwrap_or("").cmp(b["bot"].as_str().unwrap_or("")));
    ok(json!({"items": items, "total": items.len()}))
}
