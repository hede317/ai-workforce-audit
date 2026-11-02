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
    pub daily_token_limit: i64,
    pub daily_cost_limit_usd: f64,
    pub ai_cost_per_1k_usd: f64,
    pub ai_pricing_json: String,
    pub ai_timezone: String,
    pub feature_botarmy: bool,
    pub feature_openmaic: bool,
    /// 国内/海外自动发布（social-auto-upload 桥）
    pub feature_autopublish: bool,
    /// social-auto-upload 仓库根目录
    pub sau_dir: String,
    /// social-auto-upload 的 venv python 绝对路径
    pub sau_python: String,
    /// 发布默认账号名（对应 cookies/<platform>_<account>.json）
    pub sau_default_account: String,
    /// 单次发布子进程超时秒数
    pub sau_timeout_secs: u64,
    /// 海外平台（YouTube/TikTok）代理，留空=直连
    pub sau_overseas_proxy: String,
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
            daily_token_limit: envs("DAILY_TOKEN_LIMIT", "0").parse().unwrap_or(0), // 0=不限
            daily_cost_limit_usd: envs("DAILY_COST_LIMIT_USD", "0").parse().unwrap_or(0.0),
            ai_cost_per_1k_usd: envs("AI_COST_PER_1K_USD", "0").parse().unwrap_or(0.0),
            ai_pricing_json: envs("AI_PRICING_JSON", "{}"),
            ai_timezone: envs("AWF_TIMEZONE", "Asia/Shanghai"),
            feature_botarmy: envs("FEATURE_BOTARMY", "0") == "1",
            feature_openmaic: envs("FEATURE_OPENMAIC", "0") == "1",
            feature_autopublish: envs("FEATURE_AUTOPUBLISH", "0") == "1",
            sau_dir: envs("SAU_DIR", ""),
            sau_python: envs("SAU_PYTHON", ""),
            sau_default_account: envs("SAU_DEFAULT_ACCOUNT", "creator"),
            sau_timeout_secs: envs("SAU_TIMEOUT_SECS", "900").parse().unwrap_or(900),
            sau_overseas_proxy: envs("SAU_OVERSEAS_PROXY", ""),
        }
    }

    /// 自动发布是否真正可用：开关打开 + 目录 + python 都配了
    pub fn autopublish_available(&self) -> bool {
        self.feature_autopublish
            && !self.sau_dir.is_empty()
            && !self.sau_python.is_empty()
            && std::path::Path::new(&self.sau_python).exists()
    }

    pub fn ai_available(&self) -> bool {
        !self.ai_base_url.is_empty() && !self.ai_api_key.is_empty() && !self.ai_model.is_empty()
    }

    pub fn embed_available(&self) -> bool {
        !self.embed_api_base.is_empty()
            && !self.embed_api_key.is_empty()
            && !self.embed_model.is_empty()
    }

    pub fn smtp_available(&self) -> bool {
        !self.smtp_host.is_empty() && !self.smtp_user.is_empty() && !self.smtp_pass.is_empty()
    }
}
