use crate::config::Config;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState(pub Arc<Inner>);

pub struct Inner {
    pub cfg: Config,
    pub pool: PgPool,
    pub http: reqwest::Client,
    /// 禁用重定向的客户端：外部抓取专用（防重定向落入内网，审计#10）
    pub http_no_redirect: reqwest::Client,
    /// 长超时客户端：MPT出片等分钟级任务专用
    pub http_long: reqwest::Client,
    /// 带cookie存储的客户端：OpenMAIC 匿名身份（425→202 两段式）
    pub http_cookies: reqwest::Client,
}

impl AppState {
    pub async fn new(cfg: Config, pool: PgPool) -> Result<Self, anyhow::Error> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("HedeYou-AI-Workforce/0.1")
            .build()?;
        let http_no_redirect = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("HedeYou-AI-Workforce/0.1")
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        let http_long = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2500))
            .user_agent("HedeYou-AI-Workforce/0.1")
            .build()?;
        let http_cookies = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .user_agent("HedeYou-AI-Workforce/0.1")
            .cookie_store(true)
            .build()?;
        Ok(Self(Arc::new(Inner {
            cfg,
            pool,
            http,
            http_no_redirect,
            http_long,
            http_cookies,
        })))
    }

    pub fn pool(&self) -> &PgPool {
        &self.0.pool
    }
}
