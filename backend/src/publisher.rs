//! 国内/海外自动发布桥（social-auto-upload）
//!
//! 设计取舍：
//! - **不内嵌浏览器自动化**。social-auto-upload 已经把 6 个平台的登录态、选择器、
//!   上传流程处理过一遍，重写一遍只会把它的坑再踩一遍。这里只做「编排 + 留痕 + 审批」。
//! - **子进程而非 HTTP**。sau 是 Python CLI，没有 HTTP 服务。用 tokio::process 起
//!   `python sau_cli.py <platform> upload-video ...`，捕获 stdout/stderr 落库。
//! - **区域是硬边界**。国内 5 平台与海外 2 平台的账号体系、代理需求、合规要求都不同，
//!   用 PlatformSpec 表驱动，代码里不出现 if platform == "douyin" 这种分支。
//! - **发布永远走审批**。本模块只被 APPROVAL 级工具调用；dispatch 门已经在前面拦过。

use crate::config::Config;
use crate::error::AppError;
use serde_json::{json, Value};
use std::process::Stdio;
use std::time::Duration;

/// 发布区域
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
    Domestic,
    Overseas,
}

impl Region {
    pub fn as_str(self) -> &'static str {
        match self {
            Region::Domestic => "DOMESTIC",
            Region::Overseas => "OVERSEAS",
        }
    }

    pub fn parse(s: &str) -> Option<Region> {
        match s.trim().to_ascii_uppercase().as_str() {
            "DOMESTIC" | "CN" | "国内" => Some(Region::Domestic),
            "OVERSEAS" | "GLOBAL" | "海外" | "国外" => Some(Region::Overseas),
            _ => None,
        }
    }
}

/// 一个平台的发布规格。表驱动，新增平台只加一行。
#[derive(Debug, Clone, Copy)]
pub struct PlatformSpec {
    /// 对外的平台键（存库用）
    pub key: &'static str,
    /// 中文名
    pub label: &'static str,
    /// 区域
    pub region: Region,
    /// sau_cli 的平台子命令
    pub sau_platform: &'static str,
    /// 是否需要代理（海外平台在中国网络下必须走代理）
    pub needs_proxy: bool,
}

/// 全部受支持的平台。国内 5 + 海外 2。
pub const PLATFORMS: &[PlatformSpec] = &[
    PlatformSpec {
        key: "douyin",
        label: "抖音",
        region: Region::Domestic,
        sau_platform: "douyin",
        needs_proxy: false,
    },
    PlatformSpec {
        key: "bilibili",
        label: "B站",
        region: Region::Domestic,
        sau_platform: "bilibili",
        needs_proxy: false,
    },
    PlatformSpec {
        key: "kuaishou",
        label: "快手",
        region: Region::Domestic,
        sau_platform: "kuaishou",
        needs_proxy: false,
    },
    PlatformSpec {
        key: "xiaohongshu",
        label: "小红书",
        region: Region::Domestic,
        sau_platform: "xiaohongshu",
        needs_proxy: false,
    },
    PlatformSpec {
        key: "tencent",
        label: "视频号",
        region: Region::Domestic,
        sau_platform: "tencent",
        needs_proxy: false,
    },
    PlatformSpec {
        key: "youtube",
        label: "YouTube",
        region: Region::Overseas,
        sau_platform: "youtube",
        needs_proxy: true,
    },
    PlatformSpec {
        key: "tiktok",
        label: "TikTok",
        region: Region::Overseas,
        sau_platform: "tk",
        needs_proxy: true,
    },
];

pub fn find_platform(key: &str) -> Option<&'static PlatformSpec> {
    let k = key.trim().to_ascii_lowercase();
    PLATFORMS.iter().find(|p| p.key == k || p.sau_platform == k)
}

pub fn platforms_in(region: Region) -> Vec<&'static PlatformSpec> {
    PLATFORMS.iter().filter(|p| p.region == region).collect()
}

/// 一次发布的输入
pub struct PublishRequest {
    pub platform: String,
    pub account: String,
    pub file: String,
    pub title: String,
    pub desc: String,
    pub tags: Vec<String>,
    pub schedule: Option<String>,
}

/// 一次发布的结果
pub struct PublishOutcome {
    pub platform: &'static str,
    pub region: Region,
    pub account: String,
    pub ok: bool,
    /// 进程退出码；超时为 None
    pub exit_code: Option<i32>,
    /// 是否因超时被终止
    pub timed_out: bool,
    /// 是否因登录态失效被拦（cookie 过期，需要人重新扫码）
    pub needs_login: bool,
    /// 合并后的 stdout+stderr（已截断）
    pub log: String,
}

/// 从 sau 输出里识别「登录态失效」。
///
/// 上游行为：cookie 过期时 sau 不会提前报错，而是「上传前检查通过」→ 打开页面 →
/// 撞到登录表单 → Playwright 抛 strict mode violation（因为登录页有 3 个 input）。
/// 这个模式必须单独识别，否则会被当成普通失败，用户不知道该去重新扫码。
fn looks_like_login_wall(log: &str) -> bool {
    let l = strip_ansi(log).to_ascii_lowercase();
    l.contains("请输入手机号")
        || l.contains("请输入验证码")
        || l.contains("web-login-area-code-input")
        || l.contains("login")
        || l.contains("扫码登录")
        || l.contains("cookie 已失效")
}

/// 账号登录态检查结果
pub struct AccountCheck {
    pub platform: &'static str,
    pub region: Region,
    pub account: String,
    pub status: &'static str,
    pub detail: String,
}

fn require_ready(cfg: &Config) -> Result<(), AppError> {
    if !cfg.autopublish_available() {
        return Err(AppError::not_configured(
            "自动发布（FEATURE_AUTOPUBLISH=1 + SAU_DIR + SAU_PYTHON 三件套）",
        ));
    }
    Ok(())
}

/// 截断日志，避免把整段 stderr 塞进数据库
fn tail_log(bytes: &[u8], max_chars: usize) -> String {
    // 先剥 ANSI，否则数据库里存一堆 \u{1b}[38;2;...m，前端与文本匹配都被污染
    let text = strip_ansi(&String::from_utf8_lossy(bytes));
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_chars {
        return text.to_string();
    }
    let start = chars.len() - max_chars;
    format!("…[截断]…\n{}", chars[start..].iter().collect::<String>())
}

/// 校验文件在 sau 能读到的位置（必须是绝对路径，且真实存在）
fn validate_file(path: &str) -> Result<(), AppError> {
    if path.trim().is_empty() {
        return Err(AppError::bad_request("缺少 file（视频文件绝对路径）"));
    }
    let p = std::path::Path::new(path);
    if !p.is_absolute() {
        return Err(AppError::bad_request("file 必须是绝对路径"));
    }
    if !p.is_file() {
        return Err(AppError::bad_request(format!("file 不存在: {path}")));
    }
    Ok(())
}

/// 跑一次 sau 子命令，返回 (成功?, 退出码, 超时?, 合并日志)
async fn run_sau(
    cfg: &Config,
    args: &[String],
    proxy: Option<&str>,
    timeout: Duration,
) -> Result<(bool, Option<i32>, bool, String), AppError> {
    let mut cmd = tokio::process::Command::new(&cfg.sau_python);
    cmd.arg("sau_cli.py")
        .args(args)
        .current_dir(&cfg.sau_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // 超时丢弃 future 时连带杀掉子进程，避免留下僵尸 chromium
        .kill_on_drop(true)
        // 子进程不要继承后端自己的环境，避免串味；只透传必要的
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("HOME", std::env::var("HOME").unwrap_or_default())
        .env("LANG", "en_US.UTF-8")
        .env("PYTHONUNBUFFERED", "1");
    if let Some(p) = proxy {
        if !p.is_empty() {
            cmd.env("HTTPS_PROXY", p)
                .env("HTTP_PROXY", p)
                .env("https_proxy", p)
                .env("http_proxy", p);
        }
    }

    let child = cmd
        .spawn()
        .map_err(|e| AppError::internal(format!("启动 sau 子进程失败: {e}")))?;

    match tokio::time::timeout(timeout, child.wait_with_output()).await {
        Ok(Ok(out)) => {
            let mut merged = Vec::new();
            merged.extend_from_slice(&out.stdout);
            merged.extend_from_slice(&out.stderr);
            let code = out.status.code();
            Ok((out.status.success(), code, false, tail_log(&merged, 4000)))
        }
        Ok(Err(e)) => Err(AppError::internal(format!("sau 子进程异常: {e}"))),
        Err(_) => {
            // 超时：wait_with_output 的 future 被丢弃，子进程由 tokio 负责 kill_on_drop
            Ok((
                false,
                None,
                true,
                format!("发布超时（{}秒），子进程已终止", timeout.as_secs()),
            ))
        }
    }
}

/// 执行一次发布
pub async fn publish(cfg: &Config, req: &PublishRequest) -> Result<PublishOutcome, AppError> {
    require_ready(cfg)?;
    validate_file(&req.file)?;

    let spec = find_platform(&req.platform)
        .ok_or_else(|| AppError::bad_request(format!("不支持的平台: {}", req.platform)))?;

    if req.title.trim().is_empty() {
        return Err(AppError::bad_request("缺少 title"));
    }

    let mut args: Vec<String> = vec![
        spec.sau_platform.to_string(),
        "upload-video".to_string(),
        "--account".to_string(),
        req.account.clone(),
        "--file".to_string(),
        req.file.clone(),
        "--title".to_string(),
        req.title.clone(),
        "--headless".to_string(),
    ];
    if !req.desc.trim().is_empty() {
        args.push("--desc".into());
        args.push(req.desc.clone());
    }
    if !req.tags.is_empty() {
        args.push("--tags".into());
        args.push(req.tags.join(","));
    }
    if let Some(s) = req.schedule.as_ref().filter(|s| !s.trim().is_empty()) {
        args.push("--schedule".into());
        args.push(s.clone());
    }

    let proxy = if spec.needs_proxy {
        Some(cfg.sau_overseas_proxy.as_str())
    } else {
        None
    };

    let timeout = Duration::from_secs(cfg.sau_timeout_secs.max(60));
    let (ok, exit_code, timed_out, log) = run_sau(cfg, &args, proxy, timeout).await?;
    let needs_login = !ok && looks_like_login_wall(&log);

    Ok(PublishOutcome {
        platform: spec.key,
        region: spec.region,
        account: req.account.clone(),
        ok,
        exit_code,
        timed_out,
        needs_login,
        log,
    })
}

/// 批量发布：逐个平台串行执行（并发会让同一浏览器的多个 profile 打架）
pub async fn publish_many(
    cfg: &Config,
    platforms: &[String],
    base: &PublishRequest,
) -> Result<Vec<PublishOutcome>, AppError> {
    require_ready(cfg)?;
    if platforms.is_empty() {
        return Err(AppError::bad_request("platforms 不能为空"));
    }
    if platforms.len() > 8 {
        return Err(AppError::bad_request("单次最多 8 个平台"));
    }

    let mut results = Vec::new();
    for p in platforms {
        let mut one = PublishRequest {
            platform: p.clone(),
            account: base.account.clone(),
            file: base.file.clone(),
            title: base.title.clone(),
            desc: base.desc.clone(),
            tags: base.tags.clone(),
            schedule: base.schedule.clone(),
        };
        // 单平台失败不中断整批
        match publish(cfg, &one).await {
            Ok(out) => results.push(out),
            Err(e) => {
                let spec = find_platform(p);
                results.push(PublishOutcome {
                    platform: spec.map(|s| s.key).unwrap_or("unknown"),
                    region: spec.map(|s| s.region).unwrap_or(Region::Domestic),
                    account: std::mem::take(&mut one.account),
                    ok: false,
                    exit_code: None,
                    timed_out: false,
                    needs_login: false,
                    log: format!("参数或环境错误: {}", e.message),
                });
            }
        }
    }
    Ok(results)
}

/// 剥掉 ANSI 颜色码。sau 的日志带 256 色转义，直接入库会污染前端与文本匹配。
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // ESC [ ... m 形式；遇到字母结尾即结束
            if chars.peek() == Some(&'[') {
                chars.next();
                for c2 in chars.by_ref() {
                    if c2.is_ascii_alphabetic() {
                        break;
                    }
                }
                continue;
            }
        }
        out.push(c);
    }
    out
}

/// 判定账号登录态。
///
/// **注意**：`"invalid".contains("valid")` 为真，所以必须先判 invalid 再判 valid。
/// 这是本模块最容易写错的一行，单测 `classify_check_*` 专门锁住它。
fn classify_check(timed_out: bool, ok: bool, raw_log: &str) -> &'static str {
    if timed_out {
        return "TIMEOUT";
    }
    let log = strip_ansi(raw_log).to_ascii_lowercase();
    // 从后往前找最后一行有内容的输出，CLI 把结论打在末尾
    let last = log
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim()
        .to_string();

    // 顺序不能换：invalid 优先于 valid
    if last.contains("invalid")
        || last.contains("expired")
        || last.contains("missing")
        || last.contains("失效")
        || last.contains("未登录")
        || last.contains("重新登录")
    {
        return "INVALID";
    }
    if last.contains("valid") || last.contains("有效") {
        return "VALID";
    }
    if ok {
        "UNKNOWN"
    } else {
        "ERROR"
    }
}

/// 检查一个平台的登录态（只读，不改 cookie）
pub async fn check_account(
    cfg: &Config,
    platform: &str,
    account: &str,
) -> Result<AccountCheck, AppError> {
    require_ready(cfg)?;
    let spec = find_platform(platform)
        .ok_or_else(|| AppError::bad_request(format!("不支持的平台: {platform}")))?;

    let args: Vec<String> = vec![
        spec.sau_platform.to_string(),
        "check".to_string(),
        "--account".to_string(),
        account.to_string(),
    ];
    let timeout = Duration::from_secs(60);
    let (ok, _code, timed_out, log) = run_sau(cfg, &args, None, timeout).await?;

    let status = classify_check(timed_out, ok, &log);

    Ok(AccountCheck {
        platform: spec.key,
        region: spec.region,
        account: account.to_string(),
        status,
        detail: tail_log(log.as_bytes(), 600),
    })
}

/// 平台清单（给前端与 Commander 用）
pub fn platform_catalog() -> Value {
    let items: Vec<Value> = PLATFORMS
        .iter()
        .map(|p| {
            json!({
                "key": p.key,
                "label": p.label,
                "region": p.region.as_str(),
                "needs_proxy": p.needs_proxy,
            })
        })
        .collect();
    json!({
        "domestic": items.iter().filter(|i| i["region"] == "DOMESTIC").cloned().collect::<Vec<_>>(),
        "overseas": items.iter().filter(|i| i["region"] == "OVERSEAS").cloned().collect::<Vec<_>>(),
        "total": items.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_table_is_consistent() {
        // 7 个平台，国内 5 + 海外 2
        assert_eq!(PLATFORMS.len(), 7);
        assert_eq!(platforms_in(Region::Domestic).len(), 5);
        assert_eq!(platforms_in(Region::Overseas).len(), 2);
        // key 唯一
        let mut keys: Vec<&str> = PLATFORMS.iter().map(|p| p.key).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), PLATFORMS.len());
        // 海外平台必须标 needs_proxy
        for p in platforms_in(Region::Overseas) {
            assert!(p.needs_proxy, "{} 应标记需要代理", p.key);
        }
    }

    #[test]
    fn region_parsing() {
        assert_eq!(Region::parse("DOMESTIC"), Some(Region::Domestic));
        assert_eq!(Region::parse("海外"), Some(Region::Overseas));
        assert_eq!(Region::parse("bogus"), None);
    }

    #[test]
    fn platform_lookup_is_case_insensitive() {
        assert!(find_platform("DouYin").is_some());
        assert!(find_platform("douyin").is_some());
        assert!(find_platform("nope").is_none());
    }

    #[test]
    fn catalog_splits_regions() {
        let c = platform_catalog();
        assert_eq!(c["total"], 7);
        assert_eq!(c["domestic"].as_array().unwrap().len(), 5);
        assert_eq!(c["overseas"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn relative_path_rejected() {
        assert!(validate_file("relative.mp4").is_err());
        assert!(validate_file("").is_err());
    }

    #[test]
    fn tail_log_truncates() {
        let s = "a".repeat(5000);
        let out = tail_log(s.as_bytes(), 100);
        assert!(out.chars().count() < 200);
        assert!(out.contains("[截断]"));
    }

    #[test]
    fn classify_check_invalid_wins_over_valid() {
        // 回归：contains("valid") 也会命中 "invalid"，必须先判 invalid
        assert_eq!(classify_check(false, true, "invalid"), "INVALID");
        assert_eq!(classify_check(false, true, "valid"), "VALID");
        assert_eq!(
            classify_check(false, true, "INFO: cookie 已失效，得重新登录一下\ninvalid"),
            "INVALID"
        );
        assert_eq!(classify_check(false, true, "valid\n"), "VALID");
    }

    #[test]
    fn classify_check_handles_ansi_and_timeout() {
        // sau 的日志带 256 色转义
        let ansi = "\u{1b}[38;2;156;191;221mINFO\u{1b}[0m: 有效";
        assert_eq!(classify_check(false, true, ansi), "VALID");
        assert_eq!(classify_check(true, false, "anything"), "TIMEOUT");
        assert_eq!(classify_check(false, true, "no signal here"), "UNKNOWN");
        assert_eq!(classify_check(false, false, "boom"), "ERROR");
    }

    #[test]
    fn login_wall_is_detected() {
        // 真实失败日志：cookie 过期时 sau 撞到登录页，Playwright 报 3 个 input 冲突
        let real = "上传前检查通过\n小人正在赶往上传主页\nLocator.set_input_files: Error: strict mode violation: resolved to 3 elements:\n<input placeholder=\"请输入手机号\" name=\"normal-input\"/>";
        assert!(looks_like_login_wall(real), "应识别为登录失效");
        // 普通失败不应误判
        assert!(!looks_like_login_wall("视频格式不支持 mp4 codec"));
    }

    #[test]
    fn tail_log_strips_ansi_before_store() {
        // 回归：日志入库前必须剥 ANSI，否则数据库里全是 \u{1b}[38;2;...m
        let ansi = "\u{1b}[38;2;112;172;222m2026-11-02\u{1b}[0m | INFO: ok";
        let out = tail_log(ansi.as_bytes(), 1000);
        assert!(!out.contains('\u{1b}'), "入库日志仍带 ANSI: {out:?}");
        assert!(out.contains("2026-11-02"));
    }

    #[test]
    fn strip_ansi_removes_colour_codes() {
        let s = "\u{1b}[38;2;112;172;222m2026-11-02\u{1b}[0m plain";
        assert_eq!(strip_ansi(s), "2026-11-02 plain");
    }
}
