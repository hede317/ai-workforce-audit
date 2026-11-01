use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    pub admin_username: String,
    pub admin_password: String,
    pub ai_base_url: String,
    pub ai_api_key: String,
    pub ai_model: String,
    pub mpt_worker_url: String,
    pub mpt_token: String,
    pub n8n_base_url: String,
    pub n8n_webhook_token: String,
    pub media_dir: String,
    pub embed_api_base: String,
    pub embed_api_key: String,
    pub embed_model: String,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_user: String,
    pub smtp_pass: String,
    pub smtp_from: String,
    pub smtp_security: String,
    pub cors_origins: String,
    pub openmaic_base: String,
}

fn envs(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            host: envs("AWF_HOST", "0.0.0.0"),
            port: envs("AWF_PORT", "8600").parse().unwrap_or(8600),
            database_url: envs(
                "DATABASE_URL",
                "postgres://ai_workforce:ai_workforce@127.0.0.1:5434/ai_workforce",
            ),
            jwt_secret: envs("AWF_JWT_SECRET", "dev-secret-change-me"),
            admin_username: envs("AWF_ADMIN_USERNAME", "admin"),
            admin_password: envs("AWF_ADMIN_PASSWORD", "change-me-admin"),
            ai_base_url: envs("AI_BASE_URL", ""),
            ai_api_key: envs("AI_API_KEY", ""),
            ai_model: envs("AI_MODEL", ""),
            mpt_worker_url: envs("MPT_WORKER_URL", ""),
            mpt_token: envs("MPT_TOKEN", ""),
            n8n_base_url: envs("N8N_BASE_URL", ""),
            n8n_webhook_token: envs("N8N_WEBHOOK_TOKEN", ""),
            media_dir: envs("AWF_MEDIA_DIR", "./data/media"),
            embed_api_base: envs("EMBED_API_BASE", ""),
            embed_api_key: envs("EMBED_API_KEY", ""),
            embed_model: envs("EMBED_MODEL", ""),
            smtp_host: envs("SMTP_HOST", ""),
            smtp_port: envs("SMTP_PORT", "465").parse().unwrap_or(465),
            smtp_user: envs("SMTP_USER", ""),
            smtp_pass: envs("SMTP_PASS", ""),
            smtp_from: envs("SMTP_FROM", ""),
            smtp_security: envs("SMTP_SECURITY", "ssl"),
            cors_origins: envs("AWF_CORS_ORIGINS", ""),
            openmaic_base: envs("OPENMAIC_BASE", "https://open.maic.chat"),
        }
    }

    pub fn ai_available(&self) -> bool {
        !self.ai_base_url.is_empty() && !self.ai_api_key.is_empty() && !self.ai_model.is_empty()
    }

    pub fn embed_available(&self) -> bool {
        !self.embed_api_base.is_empty() && !self.embed_api_key.is_empty() && !self.embed_model.is_empty()
    }

    pub fn smtp_available(&self) -> bool {
        !self.smtp_host.is_empty() && !self.smtp_user.is_empty() && !self.smtp_pass.is_empty()
    }
}
