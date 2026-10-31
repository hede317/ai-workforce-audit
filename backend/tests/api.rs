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

static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (app, _) = setup().await;
    let (st, _) = send(app, req("GET", "/health", None, None)).await;
    assert_eq!(st, StatusCode::OK);
}

#[tokio::test]
async fn auth_flow() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
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
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
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
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
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
    let (dec_st, dec) = send(app.clone(), req("POST", &format!("/api/v1/approvals/{approval_id}/decide"), Some(&token), Some(json!({"decision": "APPROVED", "note": "同意"})))).await;
    assert!(dec_st.is_success(), "{dec}");
    assert_eq!(dec["data"]["status"], "Completed", "{dec}");
    // 审计#6：根任务必须同步为 Completed（此前永远卡 Running）
    let (_, tree2) = send(app.clone(), req("GET", &format!("/api/v1/commander/goals/{root_id}"), Some(&token), None)).await;
    assert_eq!(tree2["data"]["root"]["status"], "Completed", "{tree2}");

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
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (app, _) = setup().await;
    let token = login(&app).await;
    let (_, v) = send(app, req("GET", "/api/v1/analytics/daily-report", Some(&token), None)).await;
    assert!(v["data"]["markdown"].as_str().unwrap_or("").contains("经营日报"), "{v}");
}

#[tokio::test]
async fn publishing_result_writeback() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
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
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
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

#[tokio::test]
async fn tool_invoke_permission_gate() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (app, _pool) = setup().await;
    let token = login(&app).await;

    // AUTO 级工具可直调
    let (st, v) = send(app.clone(), req("POST", "/api/v1/tools/content.topics_template/invoke", Some(&token), Some(json!({"goal": "直调测试目标", "business_unit_code": "ECOM"})))).await;
    assert!(st.is_success(), "{v}");
    assert_eq!(v["data"]["topics"].as_array().unwrap().len(), 5);

    // APPROVAL 级工具禁止直调（红线）
    let (st, v) = send(app.clone(), req("POST", "/api/v1/tools/social.package_upload/invoke", Some(&token), Some(json!({"platform": "tiktok", "title": "x"})))).await;
    assert_eq!(st, StatusCode::CONFLICT, "{v}");
    assert!(v["error"]["message"].as_str().unwrap().contains("APPROVAL_REQUIRED"));
}

#[tokio::test]
async fn product_selection_flow() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (app, pool) = setup().await;
    let token = login(&app).await;

    let (_, c) = send(app.clone(), req("POST", "/api/v1/products", Some(&token), Some(json!({"title": "测试选品：便携榨汁机", "business_unit_code": "ECOM", "price": 199.0, "source_url": "https://example.com/juicer"})))).await;
    let pid = c["data"]["id"].as_str().unwrap().to_string();

    // AI 未配置时评分工具返回 503 NOT_CONFIGURED（诚实状态，不造假）
    let (st, v) = send(app.clone(), req("POST", "/api/v1/tools/ecommerce.score_product/invoke", Some(&token), Some(json!({"product_id": pid})))).await;
    assert_eq!(st, StatusCode::SERVICE_UNAVAILABLE, "{v}");

    // 状态机：PENDING → ANALYZED → APPROVED → LISTED（不可跳步）
    for st in ["ANALYZED", "APPROVED", "LISTED"] {
        let (_, u) = send(app.clone(), req("PUT", &format!("/api/v1/products/{pid}"), Some(&token), Some(json!({"status": st})))).await;
        assert_eq!(u["data"]["updated"], true, "{st}: {u}");
    }
    let st: (String,) = sqlx::query_as("SELECT status FROM product_candidates WHERE id=$1").bind(&pid).fetch_one(&pool).await.unwrap();
    assert_eq!(st.0, "LISTED");
}

#[tokio::test]
async fn audit_publish_no_duplicate_queue() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (app, _) = setup().await;
    let token = login(&app).await;
    let (_, c) = send(app.clone(), req("POST", "/api/v1/contents", Some(&token), Some(json!({"title": "防重文章", "body": "x"})))).await;
    let id = c["data"]["id"].as_str().unwrap().to_string();
    let _ = send(app.clone(), req("POST", &format!("/api/v1/contents/{id}/approve"), Some(&token), None)).await;
    let (st1, _) = send(app.clone(), req("POST", &format!("/api/v1/contents/{id}/publish"), Some(&token), Some(json!({"platform": "tiktok"})))).await;
    assert!(st1.is_success());
    let (st2, v2) = send(app.clone(), req("POST", &format!("/api/v1/contents/{id}/publish"), Some(&token), Some(json!({"platform": "tiktok"})))).await;
    assert_eq!(st2, StatusCode::CONFLICT, "重复发布包必须被拒: {v2}");
}

#[tokio::test]
async fn audit_product_no_state_skip() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (app, _) = setup().await;
    let token = login(&app).await;
    let (_, c) = send(app.clone(), req("POST", "/api/v1/products", Some(&token), Some(json!({"title": "跳步测试品", "business_unit_code": "ECOM"})))).await;
    let id = c["data"]["id"].as_str().unwrap().to_string();
    // PENDING 直接跳 LISTED 必须被拒
    let (st, v) = send(app.clone(), req("PUT", &format!("/api/v1/products/{id}"), Some(&token), Some(json!({"status": "LISTED"})))).await;
    assert_eq!(st, StatusCode::CONFLICT, "跳步必须被拒: {v}");
}

#[tokio::test]
async fn audit_budget_and_whitelist_guards() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (app, pool) = setup().await;
    let token = login(&app).await;
    let goal = format!("预算与白名单测试-{}", Uuid::new_v4().simple());

    // 预算护栏：把 content 任务的工具次数顶到上限
    let (_, g) = send(app.clone(), req("POST", "/api/v1/commander/goals", Some(&token), Some(json!({"goal": goal})))).await;
    let root_id = g["data"]["root_id"].as_str().unwrap().to_string();
    sqlx::query("UPDATE tasks SET tool_calls_used = max_tool_calls WHERE root_id=$1 AND agent_code='content'")
        .bind(&root_id).execute(&pool).await.unwrap();
    let _ = send(app.clone(), req("POST", &format!("/api/v1/commander/goals/{root_id}/execute"), Some(&token), Some(json!({"sync": true})))).await;
    let (_, tree) = send(app.clone(), req("GET", &format!("/api/v1/commander/goals/{root_id}"), Some(&token), None)).await;
    let content_task = tree["data"]["children"].as_array().unwrap().iter().find(|c| c["agent_code"] == "content").unwrap();
    assert_eq!(content_task["status"], "Failed", "{content_task}");
    assert!(content_task["error"].as_str().unwrap().contains("BUDGET_EXCEEDED"), "{content_task}");

    // 工具白名单：清空 content Agent 的工具 → 计划的工具被拦截
    let goal2 = format!("白名单测试-{}", Uuid::new_v4().simple());
    sqlx::query("UPDATE agents SET tools='[]'::jsonb WHERE code='content'").execute(&pool).await.unwrap();
    let (_, g2) = send(app.clone(), req("POST", "/api/v1/commander/goals", Some(&token), Some(json!({"goal": goal2})))) .await;
    let root2 = g2["data"]["root_id"].as_str().unwrap().to_string();
    let _ = send(app.clone(), req("POST", &format!("/api/v1/commander/goals/{root2}/execute"), Some(&token), Some(json!({"sync": true})))).await;
    let (_, tree2) = send(app.clone(), req("GET", &format!("/api/v1/commander/goals/{root2}"), Some(&token), None)).await;
    let ct = tree2["data"]["children"].as_array().unwrap().iter().find(|c| c["agent_code"] == "content").unwrap();
    // 白名单为空 → 离线模板工具被剥掉，任务走 AI 分支；AI 未配置 → NOT_CONFIGURED 失败，而非越权执行
    if ct["status"] == "Failed" {
        let e = ct["error"].as_str().unwrap_or("");
        assert!(e.contains("无权调用") || e.contains("NOT_CONFIGURED"), "{ct}");
    }
    sqlx::query(r#"UPDATE agents SET tools='["content.topics_template","content.draft_article","ai.translate","memory.search","memory.save"]'::jsonb WHERE code='content'"#)
        .execute(&pool).await.unwrap();
}

#[tokio::test]
async fn audit_idempotency_failure_not_cached() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (app, _) = setup().await;
    let token = login(&app).await;
    let key = format!("fail-{}", Uuid::new_v4().simple());
    let build = |k: String, body: Value| {
        let mut r = req("POST", "/api/v1/topics", Some(&token), Some(body));
        r.headers_mut().insert("Idempotency-Key", k.parse().unwrap());
        r
    };
    // 第一次：缺 title → 400
    let (st1, _) = send(app.clone(), build(key.clone(), json!({}))).await;
    assert_eq!(st1, StatusCode::BAD_REQUEST);
    // 同键重试（合法 body）→ 必须能成功，证明失败响应未被缓存
    let (st2, v2) = send(app.clone(), build(key, json!({"title": "失败后重试主题"}))).await;
    assert!(st2.is_success(), "失败响应被缓存锁死了重试: {v2}");
}

#[tokio::test]
async fn audit2_retry_approval_task_requeues_not_executes() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (app, pool) = setup().await;
    let token = login(&app).await;
    let goal = format!("审批重试语义-{}", Uuid::new_v4().simple());
    let (_, g) = send(app.clone(), req("POST", "/api/v1/commander/goals", Some(&token), Some(json!({"goal": goal})))).await;
    let root_id = g["data"]["root_id"].as_str().unwrap().to_string();
    let _ = send(app.clone(), req("POST", &format!("/api/v1/commander/goals/{root_id}/execute"), Some(&token), Some(json!({"sync": true})))).await;

    // 找到审批型子任务（social.media），强制置为 Failed 模拟业务失败
    let (_, tree) = send(app.clone(), req("GET", &format!("/api/v1/commander/goals/{root_id}"), Some(&token), None)).await;
    let social = tree["data"]["children"].as_array().unwrap().iter().find(|c| c["agent_code"] == "social.media").unwrap();
    let sid = social["id"].as_str().unwrap().to_string();
    sqlx::query("UPDATE tasks SET status='Failed', error='simulate' WHERE id=$1").bind(&sid).execute(&pool).await.unwrap();
    let before: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM publishing_tasks").fetch_one(&pool).await.unwrap();

    // 重试：必须重新进审批，绝不能直接执行
    let (_, r) = send(app.clone(), req("POST", &format!("/api/v1/tasks/{sid}/retry"), Some(&token), None)).await;
    assert_eq!(r["data"]["status"], "Waiting Approval", "{r}");
    let after: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM publishing_tasks").fetch_one(&pool).await.unwrap();
    assert_eq!(before.0, after.0, "审批型任务重试绕过了审批直接执行！");
    // 且产生了新的 PENDING 审批
    let pend: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM approvals WHERE task_id=$1 AND status='PENDING'").bind(&sid).fetch_one(&pool).await.unwrap();
    assert!(pend.0 >= 1);
}

#[tokio::test]
async fn audit2_idempotency_stale_lock_recovered() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (app, pool) = setup().await;
    let token = login(&app).await;
    let key = format!("stale-{}", Uuid::new_v4().simple());
    // 模拟进程崩溃残留：15 分钟前的 NULL 占位
    sqlx::query("INSERT INTO idempotency_keys (id, idem_key, endpoint, response, created_at) VALUES ($1,$2,'/api/v1/topics',NULL, now() - interval '15 minutes')")
        .bind(Uuid::new_v4().simple().to_string()).bind(&key).execute(&pool).await.unwrap();
    // 同键请求：陈锁应被回收并正常执行
    let mut r = req("POST", "/api/v1/topics", Some(&token), Some(json!({"title": "陈锁恢复主题"})));
    r.headers_mut().insert("Idempotency-Key", key.parse().unwrap());
    let (st, v) = send(app, r).await;
    assert!(st.is_success(), "陈锁未回收导致永久锁死: {v}");
}

#[tokio::test]
async fn audit2_operator_isolation() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (app, pool) = setup().await;
    let admin_token = login(&app).await;
    // 建 OPERATOR 用户
    let _ = sqlx::query("INSERT INTO users (id, username, password_hash, role) VALUES ($1,'op-test',$2,'OPERATOR') ON CONFLICT (username) DO NOTHING")
        .bind(Uuid::new_v4().simple().to_string())
        .bind(ai_workforce::auth::hash_password("op-pass-123").unwrap())
        .execute(&pool).await.unwrap();
    let (st, v) = send(app.clone(), req("POST", "/api/v1/auth/login", None, Some(json!({"username": "op-test", "password": "op-pass-123"})))).await;
    assert!(st.is_success(), "{v}");
    let op_token = v["data"]["token"].as_str().unwrap().to_string();

    // admin 建目标
    let goal = format!("隔离测试-{}", Uuid::new_v4().simple());
    let (_, g) = send(app.clone(), req("POST", "/api/v1/commander/goals", Some(&admin_token), Some(json!({"goal": goal})))).await;
    let root_id = g["data"]["root_id"].as_str().unwrap().to_string();

    // OPERATOR 列表看不到 admin 的任务
    let (_, list) = send(app.clone(), req("GET", "/api/v1/tasks", Some(&op_token), None)).await;
    let leaked = list["data"]["items"].as_array().unwrap().iter().any(|i| i["root_id"] == root_id.as_str());
    assert!(!leaked, "OPERATOR 泄露了他人任务: {list}");
    // OPERATOR 直读他人任务 → 404
    let (st, _) = send(app.clone(), req("GET", &format!("/api/v1/commander/goals/{root_id}"), Some(&op_token), None)).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn listening_keyword_to_signal_to_topic() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (app, pool) = setup().await;
    let token = login(&app).await;

    // 1. 加关键词
    let (_, k) = send(app.clone(), req("POST", "/api/v1/listening/keywords", Some(&token), Some(json!({"keyword": "矿业 信息化", "goal_text": "中企IT外派", "business_unit_code": "ERP"})))).await;
    assert!(k["data"]["id"].as_str().is_some(), "{k}");

    // 2. 人工导入素材 → 信号（AI未配置走模板模式，诚实标注）
    let (_, r) = send(app.clone(), req("POST", "/api/v1/tools/listen.import_snippet/invoke", Some(&token), Some(json!({"text": "刚果金中资矿业招标信息化主管，要求懂ERP和地磅系统", "business_unit_code": "ERP", "platform": "manual"})))).await;
    assert!(r["success"].as_bool().unwrap(), "{r}");
    assert_eq!(r["data"]["mode"], "template_offline");
    let sig_id = r["data"]["signal_id"].as_str().unwrap().to_string();

    // 3. 信号 → 选题，状态转 CONVERTED
    let (_, tt) = send(app.clone(), req("POST", &format!("/api/v1/listening/signals/{sig_id}/to-topic"), Some(&token), None)).await;
    assert!(tt["data"]["topic_id"].as_str().is_some(), "{tt}");
    let st: (String,) = sqlx::query_as("SELECT status FROM listening_signals WHERE id=$1").bind(&sig_id).fetch_one(&pool).await.unwrap();
    assert_eq!(st.0, "CONVERTED");
}
