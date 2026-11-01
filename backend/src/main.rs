use ai_workforce::{config::Config, routers, seed, state::AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,sqlx=warn".into()),
        )
        .init();
    let cfg = Config::from_env();
    // 审计#12：弱密钥拒绝启动（开发可用 AWF_ALLOW_INSECURE=1 显式跳过）
    if cfg.jwt_secret.len() < 32 {
        if std::env::var("AWF_ALLOW_INSECURE").as_deref() == Ok("1") {
            tracing::warn!("AWF_ALLOW_INSECURE=1：使用弱 JWT 密钥启动，仅限本机开发！");
        } else {
            eprintln!("拒绝启动：AWF_JWT_SECRET 缺失或长度 <32 字符（当前 {} 字节）。\n生产必须设置强密钥；本机开发可设 AWF_ALLOW_INSECURE=1 跳过。", cfg.jwt_secret.len());
            std::process::exit(1);
        }
    }
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        // 审计#20：连接级统一时区，消除日报跨日撕裂
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                sqlx::query("SET TIME ZONE 'Asia/Shanghai'")
                    .execute(&mut *conn)
                    .await?;
                Ok(())
            })
        })
        .connect(&cfg.database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    seed::run(&pool, &cfg).await?;
    let state = AppState::new(cfg.clone(), pool).await?;

    // 审计#13：CORS 白名单（未配置时保持 permissive 便于本机开发）
    let cors = if cfg.cors_origins.is_empty() {
        tower_http::cors::CorsLayer::permissive()
    } else {
        let origins: Vec<axum::http::HeaderValue> = cfg
            .cors_origins
            .split(',')
            .filter_map(|s| axum::http::HeaderValue::from_str(s.trim()).ok())
            .collect();
        tower_http::cors::CorsLayer::new()
            .allow_origin(origins)
            .allow_methods(tower_http::cors::Any)
            .allow_headers(tower_http::cors::Any)
    };
    let app = routers::build_router(state).layer(cors);

    let addr = format!("{}:{}", cfg.host, cfg.port);
    tracing::info!("HedeYou AI Workforce 监听 http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
