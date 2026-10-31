use ai_workforce::config::Config;
use ai_workforce::routers;
use ai_workforce::seed;
use ai_workforce::state::AppState;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

async fn setup() -> (axum::Router, PgPool) {
    let db_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ai_workforce:ai_workforce@127.0.0.1:5434/ai_workforce_test".into());
    let slash = db_url.rfind('/').unwrap();
    let db_name = db_url[slash + 1..].to_string();
    let admin_url = format!("{}/postgres", &db_url[..slash]);
    let admin = sqlx::PgPool::connect(&admin_url).await.expect("连接PG管理库失败（本机 5434 需有 postgres 实例）");
    let exists: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM pg_database WHERE datname=$1").bind(&db_name).fetch_optional(&admin).await.unwrap();
    if exists.is_none() {
        if let Err(e) = sqlx::query(&format!("CREATE DATABASE {db_name}")).execute(&admin).await {
            // 并发建库时可能撞唯一约束，视为已存在
            let dup = matches!(&e, sqlx::Error::Database(d) if matches!(d.code().as_deref(), Some("42P04") | Some("23505")));
            if !dup {
                panic!("建测试库失败: {e}");
            }
        }
    }
    let pool = sqlx::PgPool::connect(&db_url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let cfg = Config {
        admin_username: "testadmin".into(),
        admin_password: "test-pass-123".into(),
        jwt_secret: "test-secret".into(),
        ..Config::from_env()
    };
    seed::run(&pool, &cfg).await.unwrap();
    let state = AppState::new(cfg, pool.clone()).await.unwrap();
    (routers::build_router(state), pool)
}

fn req(method: &str, uri: &str, token: Option<&str>, body: Option<Value>) -> Request<Body> {
    let mut b = Request::builder().method(method).uri(uri);
    if let Some(t) = token {
        b = b.header("Authorization", format!("Bearer {t}"));
    }
    match body {
        Some(v) => b.header("content-type", "application/json").body(Body::from(v.to_string())).unwrap(),
        None => b.body(Body::empty()).unwrap(),
    }
}

async fn send(app: axum::Router, r: Request<Body>) -> (StatusCode, Value) {
    let res = app.oneshot(r).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let v = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, v)
}

async fn login(app: &axum::Router) -> String {
    let (st, v) = send(
        app.clone(),
        req("POST", "/api/v1/auth/login", None, Some(json!({"username": "testadmin", "password": "test-pass-123"}))),
    )
    .await;
    assert!(st.is_success(), "登录失败: {v}");
    v["data"]["token"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn health_ok() {
    let (app, _) = setup().await;
    let (st, _) = send(app, req("GET", "/health", None, None)).await;
    assert_eq!(st, StatusCode::OK);
}

#[tokio::test]
async fn auth_flow() {
    let (app, _) = setup().await;
    let (st, _) = send(app.clone(), req("POST", "/api/v1/auth/login", None, Some(json!({"username": "testadmin", "password": "wrong"})))).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let token = login(&app).await;
    let (st, v) = send(app, req("GET", "/api/v1/auth/me", Some(&token), None)).await;
    assert!(st.is_success());
    assert_eq!(v["data"]["username"], "testadmin");
}

#[tokio::test]
async fn business_units_seeded() {
    let (app, _) = setup().await;
    let token = login(&app).await;
    let (_, v) = send(app, req("GET", "/api/v1/business-units", Some(&token), None)).await;
    let codes: Vec<&str> = v["data"]["items"].as_array().unwrap().iter().map(|i| i["code"].as_str().unwrap()).collect();
    for c in ["EDU", "ECOM", "STUDY", "ERP"] {
        assert!(codes.contains(&c), "缺少业务单元 {c}: {v}");
    }
}

/// 端到端：目标 → 离线拆解 → 执行 → 审批门控 → 批准后执行 → 发布包落库
#[tokio::test]
async fn commander_offline_full_flow() {
    let (app, pool) = setup().await;
    let token = login(&app).await;
    let goal = format!("推广企业ERP培训业务，获取咨询线索-{}", Uuid::new_v4().simple());

    // 1. 下达目标（AI 未配置 → 离线兜底计划）
    let (st, v) = send(app.clone(), req("POST", "/api/v1/commander/goals", Some(&token), Some(json!({"goal": goal})))).await;
    assert!(st.is_success(), "{v}");
    assert_eq!(v["data"]["mode"], "offline_fallback");
    let root_id = v["data"]["root_id"].as_str().unwrap().to_string();
    assert_eq!(v["data"]["children"].as_array().unwrap().len(), 4);

    // 2. 执行
    let (st, _) = send(app.clone(), req("POST", &format!("/api/v1/commander/goals/{root_id}/execute"), Some(&token), Some(json!({"sync": true})))).await;
    assert!(st.is_success());

    // 3. 树状态：3个离线任务完成，1个需审批
    let (_, tree) = send(app.clone(), req("GET", &format!("/api/v1/commander/goals/{root_id}"), Some(&token), None)).await;
    let children = tree["data"]["children"].as_array().unwrap();
    let completed = children.iter().filter(|c| c["status"] == "Completed").count();
    let waiting = children.iter().filter(|c| c["status"] == "Waiting Approval").count();
    assert!(completed >= 3, "完成数不足: {tree}");
    assert_eq!(waiting, 1, "应有1个等待审批: {tree}");

    // 4. 审批中心批准
    let (_, ap) = send(app.clone(), req("GET", "/api/v1/approvals?status=PENDING", Some(&token), None)).await;
    let pending = ap["data"]["items"].as_array().unwrap();
    assert!(!pending.is_empty(), "应有待审批项");
    let approval_id = pending[0]["id"].as_str().unwrap().to_string();
    let (_, dec) = send(app.clone(), req("POST", &format!("/api/v1/approvals/{approval_id}/decide"), Some(&token), Some(json!({"decision": "APPROVED", "note": "同意"})))).await;
    assert!(st.is_success(), "{dec}");
    assert_eq!(dec["data"]["status"], "Completed", "{dec}");

    // 5. 发布包已入库（人工发布队列）
    let cnt: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM publishing_tasks WHERE status='HUMAN_QUEUE'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(cnt.0 >= 1, "发布包未落库");

    // 6. 任务记忆已自动沉淀
    let mem: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM memories WHERE kind='TASK' AND source_task_id IS NOT NULL")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(mem.0 >= 1, "任务记忆未沉淀");
}

#[tokio::test]
async fn daily_report_endpoint() {
    let (app, _) = setup().await;
    let token = login(&app).await;
    let (_, v) = send(app, req("GET", "/api/v1/analytics/daily-report", Some(&token), None)).await;
    assert!(v["data"]["markdown"].as_str().unwrap_or("").contains("经营日报"), "{v}");
}

#[tokio::test]
async fn publishing_result_writeback() {
    let (app, pool) = setup().await;
    let token = login(&app).await;

    // 1. 建内容 → 审批 → 生成发布包
    let (_, c) = send(app.clone(), req("POST", "/api/v1/contents", Some(&token), Some(json!({"title": "回写测试文章", "body": "正文", "business_unit_code": "EDU"})))).await;
    let content_id = c["data"]["id"].as_str().unwrap().to_string();
    let (_, a) = send(app.clone(), req("POST", &format!("/api/v1/contents/{content_id}/approve"), Some(&token), None)).await;
    assert_eq!(a["data"]["status"], "APPROVED", "{a}");
    let (_, p) = send(app.clone(), req("POST", &format!("/api/v1/contents/{content_id}/publish"), Some(&token), Some(json!({"platform": "website"})))).await;
    let pt_id = p["data"]["publishing_task_id"].as_str().unwrap().to_string();

    // 2. 未审批内容禁止发布（红线）
    let (_, c2) = send(app.clone(), req("POST", "/api/v1/contents", Some(&token), Some(json!({"title": "未审批文章", "body": "x"})))).await;
    let c2id = c2["data"]["id"].as_str().unwrap().to_string();
    let (st, _) = send(app.clone(), req("POST", &format!("/api/v1/contents/{c2id}/publish"), Some(&token), Some(json!({"platform": "tiktok"})))).await;
    assert_eq!(st, StatusCode::CONFLICT, "未审批内容发布必须被拒绝");

    // 3. 人工发布后回填结果 → PUBLISHED
    let (_, r) = send(app.clone(), req("POST", &format!("/api/v1/publishing/{pt_id}/result"), Some(&token), Some(json!({"published_url": "https://learn.hedeyou.com/post-1", "metrics": {"views": 120}})))).await;
    assert_eq!(r["data"]["status"], "PUBLISHED", "{r}");
    let (_, list) = send(app.clone(), req("GET", "/api/v1/publishing", Some(&token), None)).await;
    let item = list["data"]["items"].as_array().unwrap().iter().find(|i| i["id"] == pt_id.as_str()).unwrap();
    assert_eq!(item["results"].as_array().unwrap().len(), 1, "{list}");
    let cnt: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM publishing_results WHERE publishing_task_id=$1").bind(&pt_id).fetch_one(&pool).await.unwrap();
    assert_eq!(cnt.0, 1);
}

#[tokio::test]
async fn idempotent_topic_creation() {
    let (app, _) = setup().await;
    let token = login(&app).await;
    let key = Uuid::new_v4().simple().to_string();
    let build = |k: String| {
        let mut r = req("POST", "/api/v1/topics", Some(&token), Some(json!({"title": "幂等测试主题", "business_unit_code": "EDU"})));
        r.headers_mut().insert("Idempotency-Key", k.parse().unwrap());
        r
    };
    let (st1, v1) = send(app.clone(), build(key.clone())).await;
    assert!(st1.is_success(), "{v1}");
    let (st2, v2) = send(app, build(key)).await;
    assert!(st2.is_success());
    assert_eq!(v1["data"]["id"], v2["data"]["id"], "幂等键第二次应返回同一记录");
}
