use crate::config::Config;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState(pub Arc<Inner>);

pub struct Inner {
    pub cfg: Config,
    pub pool: PgPool,
    pub http: reqwest::Client,
}

impl AppState {
    pub async fn new(cfg: Config, pool: PgPool) -> Result<Self, anyhow::Error> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("HedeYou-AI-Workforce/0.1")
            .build()?;
        Ok(Self(Arc::new(Inner { cfg, pool, http })))
    }

    pub fn pool(&self) -> &PgPool {
        &self.0.pool
    }
}
