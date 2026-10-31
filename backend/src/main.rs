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
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .connect(&cfg.database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    seed::run(&pool, &cfg).await?;
    let state = AppState::new(cfg.clone(), pool).await?;

    let app = routers::build_router(state).layer(tower_http::cors::CorsLayer::permissive());

    let addr = format!("{}:{}", cfg.host, cfg.port);
    tracing::info!("HedeYou AI Workforce 监听 http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
