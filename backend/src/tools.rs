use crate::ai::{AiProvider, ChatMessage};
use crate::auth::new_id;
use crate::error::AppError;
use crate::state::AppState;
use axum::response::Response;
use chrono::Local;
use serde_json::{json, Value};
use crate::ai::{cosine, parse_pgvector, vector_to_pgtext, EmbedClient};
use lettre::message::header::ContentType;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

pub struct ToolCtx<'a> {
    pub state: &'a AppState,
    pub task_id: String,
    pub approved: bool,
}

pub struct ToolDef {
    pub code: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub permission_level: &'static str, // AUTO | APPROVAL
    pub input_schema: &'static str,     // JSON Schema（文本，入库时解析）
}

/// 工具注册表（Tool Registry）—— 所有工具统一登记，Agent 不得私接接口
pub const TOOL_DEFS: &[ToolDef] = &[
    ToolDef { code: "ai.generate", name: "AI文本生成", category: "ai", description: "调用统一模型网关生成文本", permission_level: "AUTO", input_schema: r#"{"prompt":"string","system":"string?"}"# },
    ToolDef { code: "ai.translate", name: "AI翻译", category: "ai", description: "多语言翻译", permission_level: "AUTO", input_schema: r#"{"text":"string","target_language":"string"}"# },
    ToolDef { code: "content.topics_template", name: "离线选题模板", category: "content", description: "基于目标关键词确定性生成5个选题（无需AI，离线可用）", permission_level: "AUTO", input_schema: r#"{"goal":"string","business_unit_code":"string?"}"# },
    ToolDef { code: "content.draft_article", name: "AI文章起草", category: "content", description: "生成SEO文章草稿并入库", permission_level: "AUTO", input_schema: r#"{"topic":"string","business_unit_code":"string?","language":"string?"}"# },
    ToolDef { code: "web.fetch", name: "网页读取", category: "web", description: "抓取网页并提取正文文本", permission_level: "AUTO", input_schema: r#"{"url":"string"}"# },
    ToolDef { code: "seo.check", name: "SEO检查", category: "seo", description: "对页面执行Title/Description/H1/Alt/Canonical/Schema/Sitemap检查", permission_level: "AUTO", input_schema: r#"{"url":"string?","html":"string?"}"# },
    ToolDef { code: "video.placeholder", name: "占位视频渲染", category: "video", description: "ffmpeg渲染5秒测试视频（验证视频管线）", permission_level: "AUTO", input_schema: r#"{"title":"string?"}"# },
    ToolDef { code: "video.render_mpt", name: "MoneyPrinterTurbo出片", category: "video", description: "转发到现有MPT出片服务", permission_level: "AUTO", input_schema: r#"{"script":"string"}"# },
    ToolDef { code: "social.package_upload", name: "社媒人工发布包", category: "social", description: "生成平台发布包进入人工发布队列", permission_level: "APPROVAL", input_schema: r#"{"platform":"string","title":"string","description":"string?","tags":"array?","media_path":"string?"}"# },
    ToolDef { code: "site.publish_wp", name: "WordPress发布", category: "site", description: "将内容发布为WordPress草稿（需业务单元配置wp凭据）", permission_level: "APPROVAL", input_schema: r#"{"business_unit_code":"string","title":"string","body":"string"}"# },
    ToolDef { code: "memory.save", name: "记忆保存", category: "memory", description: "保存记忆；ENTERPRISE类需用户确认后生效", permission_level: "AUTO", input_schema: r#"{"kind":"ENTERPRISE|USER|TASK|KNOWLEDGE","title":"string","content":"string","tags":"array?"}"# },
    ToolDef { code: "memory.search", name: "记忆检索", category: "memory", description: "按关键词检索长期记忆", permission_level: "AUTO", input_schema: r#"{"q":"string","kind":"string?","limit":"int?"}"# },
    ToolDef { code: "crm.create_lead", name: "线索创建", category: "crm", description: "写入客户线索并记录事件", permission_level: "AUTO", input_schema: r#"{"name":"string?","email":"string?","phone":"string?","business_unit_code":"string?","source_channel":"string?","summary":"string?"}"# },
    ToolDef { code: "email.send_draft", name: "邮件发送", category: "email", description: "通过SMTP发送邮件（红线：必须审批）", permission_level: "APPROVAL", input_schema: r#"{"to":"string","subject":"string","body":"string"}"# },
    ToolDef { code: "ecommerce.score_product", name: "选品AI评分", category: "ecommerce", description: "对选品候选做需求/竞争/毛利分析并回写评分（需AI）", permission_level: "AUTO", input_schema: r#"{"product_id":"string"}"# },
    ToolDef { code: "data.daily_report", name: "经营日报", category: "data", description: "汇总当日经营数据生成Markdown日报", permission_level: "AUTO", input_schema: r#"{}"# },
];

fn tool_permission(code: &str) -> &'static str {
    TOOL_DEFS.iter().find(|t| t.code == code).map(|t| t.permission_level).unwrap_or("AUTO")
}

pub fn is_valid_tool(code: &str) -> bool {
    TOOL_DEFS.iter().any(|t| t.code == code)
}

pub async fn dispatch(code: &str, args: Value, ctx: &ToolCtx<'_>) -> Result<Value, AppError> {
    if !is_valid_tool(code) {
        return Err(AppError::bad_request(format!("未注册的工具: {code}")));
    }
    if tool_permission(code) == "APPROVAL" && !ctx.approved {
        return Err(AppError::forbidden("APPROVAL_REQUIRED: 该工具需要人工审批后才能执行"));
    }
    let state = ctx.state;
    match code {
        "ai.generate" => {
            let prompt = args.get("prompt").and_then(|v| v.as_str()).ok_or_else(|| AppError::bad_request("缺少 prompt"))?;
            let system = args.get("system").and_then(|v| v.as_str()).unwrap_or("你是高效的企业营销与经营助手。");
            let reply = call_ai(state, system, prompt, 2000).await?;
            Ok(json!({"output": reply.text, "tokens": reply.tokens, "mode": "ai"}))
        }
        "ai.translate" => {
            let text = args.get("text").and_then(|v| v.as_str()).ok_or_else(|| AppError::bad_request("缺少 text"))?;
            let lang = args.get("target_language").and_then(|v| v.as_str()).unwrap_or("en");
            let reply = call_ai(state, "你是专业翻译，只输出译文。", &format!("翻译为{lang}：\n{text}"), 2000).await?;
            Ok(json!({"output": reply.text, "tokens": reply.tokens, "mode": "ai"}))
        }
        "content.topics_template" => offline_topics(state, &args).await,
        "content.draft_article" => draft_article(state, ctx, &args).await,
        "web.fetch" => web_fetch(state, &args).await,
        "seo.check" => seo_check(state, &args).await,
        "video.placeholder" => video_placeholder(state, &args).await,
        "video.render_mpt" => video_mpt(state, &args).await,
        "social.package_upload" => social_package(state, &args).await,
        "site.publish_wp" => site_publish_wp(state, &args).await,
        "memory.save" => memory_save(state, &args).await,
        "memory.search" => memory_search(state, &args).await,
        "crm.create_lead" => crm_create_lead(state, &args).await,
        "email.send_draft" => email_send(state, &args).await,
        "ecommerce.score_product" => score_product(state, &args).await,
        "data.daily_report" => daily_report(state).await,
        _ => Err(AppError::bad_request(format!("工具 {code} 未实现"))),
    }
}

async fn call_ai(state: &AppState, system: &str, user: &str, max_tokens: u64) -> Result<crate::ai::ChatReply, AppError> {
    if !state.0.cfg.ai_available() {
        return Err(AppError::not_configured("AI 模型网关 (AI_BASE_URL/AI_API_KEY/AI_MODEL)"));
    }
    let provider = AiProvider {
        base_url: &state.0.cfg.ai_base_url,
        api_key: &state.0.cfg.ai_api_key,
        model: &state.0.cfg.ai_model,
        http: &state.0.http,
    };
    provider.chat(&[ChatMessage { role: "system".into(), content: system.into() }, ChatMessage { role: "user".into(), content: user.into() }], max_tokens).await
}

/// 审计#10：受限地址判定（std 的 is_private/is_link_local 尚未稳定，手动实现）
fn is_restricted_ip(ip: &std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            v4.is_loopback() || v4.is_private() || v4.is_link_local() || v4.is_unspecified() || v4.is_broadcast() || v4.is_multicast() || v4.is_documentation()
        }
        std::net::IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || (v6.segments()[0] & 0xfe00) == 0xfc00 // fc00::/7 唯一本地
                || (v6.segments()[0] & 0xffc0) == 0xfe80 // fe80::/10 链路本地
        }
    }
}

/// 审计#10：外链 SSRF 防护——仅 http/https，解析后拒绝环回/内网/链路本地地址
async fn validate_external_url(state: &AppState, raw: &str) -> Result<String, AppError> {
    let parsed = reqwest::Url::parse(raw).map_err(|_| AppError::bad_request("URL 无法解析"))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(AppError::bad_request("仅允许 http/https 协议"));
    }
    let host = parsed.host_str().ok_or_else(|| AppError::bad_request("URL 缺少主机名"))?.to_string();
    let port = parsed.port_or_known_default().unwrap_or(80);
    let addrs: Vec<std::net::IpAddr> = if let Ok(ip) = host.parse() {
        vec![ip]
    } else {
        tokio::net::lookup_host((host.as_str(), port))
            .await
            .map_err(|_| AppError::bad_request(format!("域名解析失败: {host}")))?
            .map(|s| s.ip())
            .collect()
    };
    for ip in &addrs {
        if is_restricted_ip(ip) {
            return Err(AppError::forbidden(format!("SSRF 防护：拒绝访问内网/保留地址 {ip}")));
        }
    }
    Ok(raw.to_string())
}

/// 审计#10：限量抓取（重定向禁用 + 512KB 上限），防 OOM
async fn fetch_limited(state: &AppState, url: &str, max: usize) -> Result<(u16, String), AppError> {
    let safe_url = validate_external_url(state, url).await?;
    let resp = state
        .0
        .http_no_redirect
        .get(&safe_url)
        .send()
        .await
        .map_err(|e| AppError::internal(format!("网页抓取失败: {e}")))?;
    let status = resp.status().as_u16();
    if let Some(len) = resp.content_length() {
        if len as usize > max {
            return Err(AppError::bad_request(format!("响应体 {len} 字节超过 {max} 上限")));
        }
    }
    let mut body: Vec<u8> = Vec::new();
    let mut resp = resp;
    while let Some(chunk) = resp.chunk().await.map_err(|e| AppError::internal(format!("读取响应失败: {e}")))? {
        body.extend_from_slice(&chunk);
        if body.len() >= max {
            body.truncate(max);
            break;
        }
    }
    Ok((status, String::from_utf8_lossy(&body).to_string()))
}

fn take_str(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// 二审P2-2：LIKE 通配符全量转义（% _ \）
pub(crate) fn escape_like(input: &str) -> String {
    input.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

fn truncate(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

// ---------- 工具实现 ----------

async fn offline_topics(state: &AppState, args: &Value) -> Result<Value, AppError> {
    let goal = take_str(args, "goal").unwrap_or_default();
    let bu = take_str(args, "business_unit_code");
    let base = truncate(goal.trim(), 40);
    if base.is_empty() {
        return Err(AppError::bad_request("缺少 goal"));
    }
    let angles = [
        ("入门指南", "新手常见误区与正确路径"),
        ("避坑清单", "90%的人踩过的坑与规避方法"),
        ("案例拆解", "一个真实客户的完整过程复盘"),
        ("工具与流程", "用什么工具、按什么流程做最高效"),
        ("成本与回报", "投入多少、多久见效、怎么算账"),
    ];
    let mut created = Vec::new();
    for (angle, desc) in angles {
        let title = format!("{base}·{angle}：{desc}");
        let id = new_id();
        sqlx::query("INSERT INTO content_topics (id, business_unit_code, title, keywords, language, status, priority) VALUES ($1,$2,$3,$4,$5,$6,$7)")
            .bind(&id)
            .bind(bu.as_deref().unwrap_or("ERP"))
            .bind(&title)
            .bind(json!([base, angle]))
            .bind("zh")
            .bind("NEW")
            .bind(5)
            .execute(state.pool())
            .await?;
        created.push(json!({"id": id, "title": title}));
    }
    Ok(json!({"output": format!("已生成{}个选题（离线模板模式）", created.len()), "topics": created, "mode": "template_offline"}))
}

async fn draft_article(state: &AppState, ctx: &ToolCtx<'_>, args: &Value) -> Result<Value, AppError> {
    let topic = take_str(args, "topic").ok_or_else(|| AppError::bad_request("缺少 topic"))?;
    let lang = take_str(args, "language").unwrap_or_else(|| "zh".into());
    let reply = call_ai(state, "你是资深内容与SEO作者，输出结构完整的文章（含标题/H2小节/FAQ）。", &format!("围绕选题写一篇800-1200字文章，语言：{lang}。选题：{topic}"), 3000).await?;
    let id = new_id();
    let title = reply.text.lines().next().unwrap_or(&topic).trim_start_matches('#').trim().to_string();
    sqlx::query("INSERT INTO content_assets (id, business_unit_code, content_type, title, body, language, status, source_task_id) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)")
        .bind(&id)
        .bind(take_str(args, "business_unit_code").unwrap_or_default())
        .bind("article")
        .bind(&title)
        .bind(&reply.text)
        .bind(&lang)
        .bind("DRAFT")
        .bind(&ctx.task_id)
        .execute(state.pool())
        .await?;
    Ok(json!({"output": format!("文章草稿已入库: {title}"), "content_id": id, "tokens": reply.tokens, "mode": "ai"}))
}

fn strip_html(html: &str) -> String {
    let doc = scraper::Html::parse_document(html);
    let mut text = String::new();
    for node in doc.tree.nodes() {
        if let Some(t) = node.value().as_text() {
            text.push_str(t.text.as_ref());
            text.push(' ');
        }
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

async fn web_fetch(state: &AppState, args: &Value) -> Result<Value, AppError> {
    let url = take_str(args, "url").ok_or_else(|| AppError::bad_request("缺少 url"))?;
    let (status, html) = fetch_limited(state, &url, 512 * 1024).await?;
    let title = scraper::Html::parse_document(&html)
        .select(&scraper::Selector::parse("title").unwrap())
        .next()
        .map(|t| t.text().collect::<String>())
        .unwrap_or_default();
    Ok(json!({"url": url, "status": status, "title": title.trim(), "text": truncate(&strip_html(&html), 8000)}))
}

async fn seo_check(state: &AppState, args: &Value) -> Result<Value, AppError> {
    let html = if let Some(url) = take_str(args, "url") {
        fetch_limited(state, &url, 512 * 1024).await?.1
    } else {
        take_str(args, "html").ok_or_else(|| AppError::bad_request("需要 url 或 html"))?
    };
    let url_for_sitemap = take_str(args, "url");
    // scraper::Html 非 Send：所有解析在局部作用域完成，不跨 await
    let mut checks = {
        let doc = scraper::Html::parse_document(&html);
        let sel = |s: &str| scraper::Selector::parse(s).unwrap();
        let title = doc.select(&sel("title")).next().map(|t| t.text().collect::<String>().trim().to_string());
        let desc = doc.select(&sel("meta[name=description]")).next().and_then(|m| m.value().attr("content").map(|s| s.to_string()));
        let h1_count = doc.select(&sel("h1")).count();
        let imgs: Vec<_> = doc.select(&sel("img")).collect();
        let imgs_no_alt = imgs.iter().filter(|i| i.value().attr("alt").map(|a| a.trim().is_empty()).unwrap_or(true)).count();
        let has_canonical = doc.select(&sel("link[rel=canonical]")).next().is_some();
        let has_schema = html.contains("application/ld+json");
        vec![
            json!({"item": "Title", "ok": title.as_deref().map(|t| !t.is_empty() && t.chars().count() <= 60).unwrap_or(false)}),
            json!({"item": "MetaDescription", "ok": desc.as_deref().map(|d| !d.is_empty() && d.chars().count() <= 160).unwrap_or(false)}),
            json!({"item": "H1唯一", "ok": h1_count == 1}),
            json!({"item": "图片Alt", "ok": imgs_no_alt == 0, "detail": format!("{}/{} 缺失", imgs_no_alt, imgs.len())}),
            json!({"item": "Canonical", "ok": has_canonical}),
            json!({"item": "Schema结构化数据", "ok": has_schema}),
        ]
    };
    if let Some(url) = url_for_sitemap {
        let base = url.trim_end_matches('/');
        let sm = state.0.http_no_redirect.get(format!("{base}/sitemap.xml")).send().await;
        checks.push(json!({"item": "Sitemap可访问", "ok": sm.map(|r| r.status().is_success()).unwrap_or(false)}));
    }
    let total = checks.len();
    let passed = checks.iter().filter(|c| c["ok"].as_bool().unwrap_or(false)).count();
    Ok(json!({"score": (passed * 100 / total), "checks": checks, "mode": "deterministic"}))
}

async fn video_placeholder(state: &AppState, args: &Value) -> Result<Value, AppError> {
    let title = take_str(args, "title").unwrap_or_else(|| "AI Workforce".into());
    let ffmpeg_ok = tokio::process::Command::new("ffmpeg").arg("-version").output().await.map(|o| o.status.success()).unwrap_or(false);
    if !ffmpeg_ok {
        return Err(AppError::not_configured("ffmpeg（本机未安装或不在PATH）"));
    }
    let name = format!("{}.mp4", new_id());
    let dir = std::path::Path::new(&state.0.cfg.media_dir);
    std::fs::create_dir_all(dir).map_err(|e| AppError::internal(format!("创建媒体目录失败: {e}")))?;
    let path = dir.join(&name);
    let out = tokio::process::Command::new("ffmpeg")
        .args(["-y", "-f", "lavfi", "-i", "testsrc=duration=5:size=1280x720:rate=25", "-pix_fmt", "yuv420p"])
        .arg(&path)
        .output()
        .await
        .map_err(|e| AppError::internal(format!("ffmpeg执行失败: {e}")))?;
    if !out.status.success() {
        return Err(AppError::internal(format!("ffmpeg渲染失败: {}", String::from_utf8_lossy(&out.stderr))));
    }
    Ok(json!({"output": format!("占位视频已渲染: {}（{}）", path.display(), truncate(&title, 20)), "media_path": path.display().to_string(), "mode": "local_ffmpeg"}))
}

async fn video_mpt(state: &AppState, args: &Value) -> Result<Value, AppError> {
    let url = state.0.cfg.mpt_worker_url.trim_end_matches('/');
    if url.is_empty() {
        return Err(AppError::not_configured("MPT出片服务 (MPT_WORKER_URL)"));
    }
    let parsed = reqwest::Url::parse(&format!("{url}/render_mpt")).map_err(|_| AppError::bad_request("MPT_WORKER_URL 无法解析"))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(AppError::bad_request("MPT_WORKER_URL 仅允许 http/https"));
    }
    let resp = state.0.http_no_redirect.post(format!("{url}/render_mpt")).json(args).send().await
        .map_err(|e| AppError::internal(format!("MPT服务连接失败: {e}")))?;
    let payload: Value = resp.json().await.unwrap_or(json!({}));
    Ok(json!({"output": "已提交MPT出片任务", "provider_response": payload, "mode": "mpt"}))
}

async fn social_package(state: &AppState, args: &Value) -> Result<Value, AppError> {
    let platform = take_str(args, "platform").ok_or_else(|| AppError::bad_request("缺少 platform"))?;
    let title = take_str(args, "title").unwrap_or_else(|| "未命名".into());
    let id = new_id();
    let payload = json!({
        "title": title,
        "description": take_str(args, "description").unwrap_or_default(),
        "tags": args.get("tags").cloned().unwrap_or(json!([])),
        "media_path": take_str(args, "media_path").unwrap_or_default(),
        "checklist": ["确认账号授权状态", "按平台格式核对画幅/时长", "AI生成内容按平台要求标注"],
    });
    sqlx::query("INSERT INTO publishing_tasks (id, platform, status, payload) VALUES ($1,$2,$3,$4)")
        .bind(&id).bind(&platform).bind("HUMAN_QUEUE")
        .bind(payload)
        .execute(state.pool()).await?;
    Ok(json!({"output": format!("已生成 {platform} 人工发布包（人工上传后回填链接）"), "publishing_task_id": id, "mode": "human_queue"}))
}

async fn site_publish_wp(state: &AppState, args: &Value) -> Result<Value, AppError> {
    let bu_code = take_str(args, "business_unit_code").ok_or_else(|| AppError::bad_request("缺少 business_unit_code"))?;
    let bu: Option<(Option<serde_json::Value>,)> =
        sqlx::query_as("SELECT configuration FROM business_units WHERE code = $1")
            .bind(&bu_code).fetch_optional(state.pool()).await?;
    let wp = bu.and_then(|(c,)| c).and_then(|c| c.get("wp").cloned()).unwrap_or(Value::Null);
    let base = wp.get("base_url").and_then(|v| v.as_str()).unwrap_or_default().to_string();
    if base.is_empty() {
        return Err(AppError::not_configured(&format!("业务单元 {bu_code} 的 WordPress 配置（configuration.wp.base_url/username/app_password）")));
    }
    let title = take_str(args, "title").unwrap_or_else(|| "未命名".into());
    let body = take_str(args, "body").unwrap_or_default();
    let parsed = reqwest::Url::parse(&base).map_err(|_| AppError::bad_request("wp.base_url 无法解析"))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(AppError::bad_request("wp.base_url 仅允许 http/https"));
    }
    let resp = state.0.http_no_redirect.post(format!("{}/wp-json/wp/v2/posts", base.trim_end_matches('/')))
        .basic_auth(wp.get("username").and_then(|v| v.as_str()).unwrap_or(""), Some(wp.get("app_password").and_then(|v| v.as_str()).unwrap_or("")))
        .json(&json!({"title": title, "content": body, "status": "draft"}))
        .send().await.map_err(|e| AppError::internal(format!("WordPress连接失败: {e}")))?;
    let payload: Value = resp.json().await.unwrap_or(json!({}));
    Ok(json!({"output": format!("已在WordPress创建草稿（ID: {}）", payload["id"].as_i64().unwrap_or(0)), "wp_post_id": payload["id"].clone(), "link": payload["link"].clone(), "mode": "wp_draft"}))
}

async fn memory_save(state: &AppState, args: &Value) -> Result<Value, AppError> {
    let kind = take_str(args, "kind").unwrap_or_else(|| "TASK".into());
    if !["ENTERPRISE", "USER", "TASK", "KNOWLEDGE"].contains(&kind.as_str()) {
        return Err(AppError::bad_request("kind 必须为 ENTERPRISE/USER/TASK/KNOWLEDGE"));
    }
    let title = take_str(args, "title").ok_or_else(|| AppError::bad_request("缺少 title"))?;
    let content = take_str(args, "content").ok_or_else(|| AppError::bad_request("缺少 content"))?;
    // 企业记忆不得由 Agent 直接写入生效（规范4.2）：进入 PENDING 待用户确认
    let status = if kind == "ENTERPRISE" { "PENDING" } else { "ACTIVE" };
    let id = new_id();
    sqlx::query("INSERT INTO memories (id, kind, title, content, tags, source_task_id, status) VALUES ($1,$2,$3,$4,$5,$6,$7)")
        .bind(&id).bind(&kind).bind(truncate(&title, 200)).bind(truncate(&content, 5000))
        .bind(args.get("tags").cloned().unwrap_or(json!([])))
        .bind(take_str(args, "source_task_id").unwrap_or_default())
        .bind(status)
        .execute(state.pool()).await?;
    embed_memory(state, &id, &title, &content).await;
    Ok(json!({"output": format!("记忆已保存（{kind}/{status}）"), "memory_id": id}))
}

async fn memory_search(state: &AppState, args: &Value) -> Result<Value, AppError> {
    let q = take_str(args, "q").unwrap_or_default();
    let kind = take_str(args, "kind");
    // 二审P2-3：limit 钳位，负数/超大值不再穿透
    let limit: i64 = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(5).clamp(1, 50);
    // 向量语义检索优先（配置了 EMBED_* 时）
    if state.0.cfg.embed_available() && !q.trim().is_empty() {
        let client = EmbedClient {
            base_url: &state.0.cfg.embed_api_base,
            api_key: &state.0.cfg.embed_api_key,
            model: &state.0.cfg.embed_model,
            http: &state.0.http,
        };
        if let Ok(qvec) = client.embed(&q).await {
            // 审计#15：pgvector 库内 <=> 余弦排序，全量历史可检索、不拉向量回应用层
            let pg_vec = vector_to_pgtext(&qvec);
            let rows: Vec<(String, String, String, String, f64)> = sqlx::query_as(
                "SELECT id, kind, title, left(content,300), (1.0 - (embedding <=> $1::text::vector)) AS score FROM memories WHERE status='ACTIVE' AND embedding IS NOT NULL AND ($2::text IS NULL OR kind=$2) AND (1.0 - (embedding <=> $1::text::vector)) > 0.25 ORDER BY embedding <=> $1::text::vector LIMIT $3",
            )
            .bind(&pg_vec)
            .bind(kind.as_deref())
            .bind(limit)
            .fetch_all(state.pool())
            .await?;
            let items: Vec<Value> = rows
                .into_iter()
                .map(|(id, k, title, excerpt, score)| json!({"id": id, "kind": k, "title": title, "excerpt": excerpt, "score": score}))
                .collect();
            if !items.is_empty() {
                return Ok(json!({"output": format!("向量检索到{}条相关记忆", items.len()), "memories": items, "mode": "vector_search"}));
            }
        }
    }
    let like = format!("%{}%", escape_like(&q));
    let rows: Vec<(String, String, String, String)> = if let Some(k) = kind {
        sqlx::query_as("SELECT id, kind, title, left(content, 300) FROM memories WHERE status='ACTIVE' AND kind=$1 AND (title LIKE $2 ESCAPE '\\' OR content LIKE $2 ESCAPE '\\') ORDER BY created_at DESC LIMIT $3")
            .bind(k).bind(&like).bind(limit).fetch_all(state.pool()).await?
    } else {
        sqlx::query_as("SELECT id, kind, title, left(content, 300) FROM memories WHERE status='ACTIVE' AND (title LIKE $1 OR content LIKE $1) ORDER BY created_at DESC LIMIT $2")
            .bind(&like).bind(limit).fetch_all(state.pool()).await?
    };
    let items: Vec<Value> = rows.iter().map(|(id, kind, title, content)| json!({"id": id, "kind": kind, "title": title, "excerpt": content})).collect();
    Ok(json!({"output": format!("检索到{}条相关记忆", items.len()), "memories": items, "mode": "like_search"}))
}

async fn crm_create_lead(state: &AppState, args: &Value) -> Result<Value, AppError> {
    // 审计#8：trim+小写规范化，空串归 None，避免空串重复崩溃
    let email = take_str(args, "email").map(|e| e.trim().to_lowercase()).filter(|e| !e.is_empty());
    if let Some(e) = &email {
        let dup: Option<(String,)> = sqlx::query_as("SELECT id FROM leads WHERE lower(btrim(email)) = $1").bind(e).fetch_optional(state.pool()).await?;
        if dup.is_some() {
            return Err(AppError::conflict(format!("该邮箱线索已存在，拒绝重复创建: {e}")));
        }
    }
    let id = new_id();
    sqlx::query("INSERT INTO leads (id, business_unit_code, name, email, phone, country, source_channel, summary) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)")
        .bind(&id)
        .bind(take_str(args, "business_unit_code").unwrap_or_default())
        .bind(take_str(args, "name").unwrap_or_default())
        .bind(email.clone())
        .bind(take_str(args, "phone").unwrap_or_default())
        .bind(take_str(args, "country").unwrap_or_default())
        .bind(take_str(args, "source_channel").unwrap_or_else(|| "agent".into()))
        .bind(take_str(args, "summary").unwrap_or_default())
        .execute(state.pool()).await?;
    sqlx::query("INSERT INTO lead_events (id, lead_id, event_type, note) VALUES ($1,$2,$3,$4)")
        .bind(new_id()).bind(&id).bind("inquiry").bind("由 Agent 创建").execute(state.pool()).await?;
    Ok(json!({"output": "线索已入库", "lead_id": id}))
}

async fn daily_report(state: &AppState) -> Result<Value, AppError> {
    let today = Local::now().format("%Y-%m-%d").to_string();
    let count = |sql: &str| {
        let pool = state.pool().clone();
        let sql = sql.to_string();
        async move {
            let r: (i64,) = sqlx::query_as(&sql).fetch_one(&pool).await.unwrap_or((0,));
            r.0
        }
    };
    let tasks_new = count("SELECT COUNT(*) FROM tasks WHERE created_at::date = CURRENT_DATE AND depth > 0").await;
    let done = count("SELECT COUNT(*) FROM tasks WHERE status='Completed' AND finished_at::date = CURRENT_DATE").await;
    let failed = count("SELECT COUNT(*) FROM tasks WHERE status='Failed' AND created_at::date = CURRENT_DATE").await;
    let topics = count("SELECT COUNT(*) FROM content_topics WHERE created_at::date = CURRENT_DATE").await;
    let leads = count("SELECT COUNT(*) FROM leads WHERE created_at::date = CURRENT_DATE").await;
    let pending_approvals = count("SELECT COUNT(*) FROM approvals WHERE status='PENDING'").await;
    let md = format!(
        "# AI Workforce 经营日报 {today}\n\n- 新增任务：{tasks_new}\n- 完成任务：{done}\n- 失败任务：{failed}\n- 新增选题：{topics}\n- 新增线索：{leads}\n- 待审批：{pending_approvals}\n"
    );
    Ok(json!({"output": format!("日报已生成（{today}）"), "markdown": md, "date": today, "mode": "deterministic"}))
}

async fn email_send(state: &AppState, args: &Value) -> Result<Value, AppError> {
    if !state.0.cfg.smtp_available() {
        return Err(AppError::not_configured("SMTP (SMTP_HOST/SMTP_PORT/SMTP_USER/SMTP_PASS/SMTP_FROM/SMTP_SECURITY)"));
    }
    let to = take_str(args, "to").ok_or_else(|| AppError::bad_request("缺少 to"))?;
    let subject = take_str(args, "subject").unwrap_or_else(|| "(无主题)".into());
    let body = take_str(args, "body").unwrap_or_default();
    let cfg = &state.0.cfg;
    let from: Mailbox = cfg.smtp_from.parse().map_err(|e| AppError::internal(format!("发件人地址无效: {e}")))?;
    let to_mb: Mailbox = to.parse().map_err(|e| AppError::bad_request(format!("收件人地址无效: {e}")))?;
    let email = Message::builder()
        .from(from)
        .to(to_mb)
        .subject(&subject)
        .header(ContentType::TEXT_PLAIN)
        .body(body)
        .map_err(|e| AppError::internal(format!("邮件构造失败: {e}")))?;
    // 审计#11：默认走证书校验的 relay/starttls_relay；仅显式 security=none 才用 builder_dangerous
    let creds = Credentials::new(cfg.smtp_user.clone(), cfg.smtp_pass.clone());
    let mailer = match cfg.smtp_security.as_str() {
        "none" => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&cfg.smtp_host)
            .port(cfg.smtp_port)
            .credentials(creds)
            .build(),
        "starttls" => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&cfg.smtp_host)
            .map_err(|e| AppError::internal(format!("TLS初始化失败: {e}")))?
            .port(cfg.smtp_port)
            .credentials(creds)
            .build(),
        _ => AsyncSmtpTransport::<Tokio1Executor>::relay(&cfg.smtp_host)
            .map_err(|e| AppError::internal(format!("TLS初始化失败: {e}")))?
            .port(cfg.smtp_port)
            .credentials(creds)
            .build(),
    };
    mailer.send(email).await.map_err(|e| AppError::internal(format!("邮件发送失败: {e}")))?;
    Ok(json!({"output": format!("邮件已发送至 {to}"), "mode": "smtp"}))
}

async fn score_product(state: &AppState, args: &Value) -> Result<Value, AppError> {
    let pid = take_str(args, "product_id").ok_or_else(|| AppError::bad_request("缺少 product_id"))?;
    let row: Option<(String, Option<String>, f64, Option<String>)> =
        sqlx::query_as("SELECT title, source_url, price, decision_note FROM product_candidates WHERE id=$1")
            .bind(&pid).fetch_optional(state.pool()).await?;
    let (title, source_url, price, note) = row.ok_or_else(|| AppError::not_found("选品不存在"))?;
    let reply = call_ai(
        state,
        "你是跨境电商选品分析师。对商品做客观分析，只输出JSON。",
        &format!("商品：{title}\n参考链接：{}\n售价：{price}\n既有备注：{}\n\n输出JSON：{{\"demand_score\":1-10需求分,\"competition_score\":1-10竞争分,\"est_cost\":估算成本,\"est_margin_pct\":估算毛利率百分比,\"decision_note\":\"三行以内结论\"}}",
            source_url.as_deref().unwrap_or("无"), note.as_deref().unwrap_or("无")),
        1200,
    ).await?;
    let parsed = crate::ai::extract_json(&reply.text).ok_or_else(|| AppError::internal("选品分析结果不是有效JSON"))?;
    sqlx::query("UPDATE product_candidates SET demand_score=$2, competition_score=$3, est_cost=$4, est_margin_pct=$5, decision_note=$6, status=CASE WHEN status='PENDING' THEN 'ANALYZED' ELSE status END, updated_at=now() WHERE id=$1")
        .bind(&pid)
        .bind(parsed.get("demand_score").and_then(|v| v.as_i64()).unwrap_or(0) as i32)
        .bind(parsed.get("competition_score").and_then(|v| v.as_i64()).unwrap_or(0) as i32)
        .bind(parsed.get("est_cost").and_then(|v| v.as_f64()).unwrap_or(0.0))
        .bind(parsed.get("est_margin_pct").and_then(|v| v.as_f64()).unwrap_or(0.0))
        .bind(parsed.get("decision_note").and_then(|v| v.as_str()).unwrap_or(""))
        .execute(state.pool())
        .await?;
    Ok(json!({"output": format!("选品评分已回写：{title}"), "analysis": parsed, "tokens": reply.tokens, "mode": "ai"}))
}

/// 审计#9-2：为指定记忆生成并写回 embedding（手动新增/修改记忆同样可被语义检索）
pub async fn embed_memory(state: &AppState, memory_id: &str, title: &str, content: &str) {
    if !state.0.cfg.embed_available() {
        return;
    }
    let client = EmbedClient {
        base_url: &state.0.cfg.embed_api_base,
        api_key: &state.0.cfg.embed_api_key,
        model: &state.0.cfg.embed_model,
        http: &state.0.http,
    };
    if let Ok(vec) = client.embed(&format!("{title}\n{content}")).await {
        let _ = sqlx::query("UPDATE memories SET embedding=$2::text::vector WHERE id=$1")
            .bind(memory_id)
            .bind(vector_to_pgtext(&vec))
            .execute(state.pool())
            .await;
    }
}
