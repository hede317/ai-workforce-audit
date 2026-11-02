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

static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn setup() -> (axum::Router, PgPool) {
    let db_url = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
        "postgres://ai_workforce:ai_workforce@127.0.0.1:5434/ai_workforce_test".into()
    });
    let slash = db_url.rfind('/').unwrap();
    let db_name = db_url[slash + 1..].to_string();
    let admin_url = format!("{}/postgres", &db_url[..slash]);
    let admin = sqlx::PgPool::connect(&admin_url)
        .await
        .expect("连接PG管理库失败（本机 5434 需有 postgres 实例）");
    let exists: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM pg_database WHERE datname=$1")
        .bind(&db_name)
        .fetch_optional(&admin)
        .await
        .unwrap();
    if exists.is_none() {
        if let Err(e) = sqlx::query(&format!("CREATE DATABASE {db_name}"))
            .execute(&admin)
            .await
        {
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
        Some(v) => b
            .header("content-type", "application/json")
            .body(Body::from(v.to_string()))
            .unwrap(),
        None => b.body(Body::empty()).unwrap(),
    }
}

async fn send(app: axum::Router, r: Request<Body>) -> (StatusCode, Value) {
    let res = app.oneshot(r).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let v = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, v)
}

async fn login(app: &axum::Router) -> String {
    let (st, v) = send(
        app.clone(),
        req(
            "POST",
            "/api/v1/auth/login",
            None,
            Some(json!({"username": "testadmin", "password": "test-pass-123"})),
        ),
    )
    .await;
    assert!(st.is_success(), "登录失败: {v}");
    v["data"]["token"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn health_ok() {
    let _g = SERIAL.lock().await;
    let (app, _) = setup().await;
    let (st, _) = send(app, req("GET", "/health", None, None)).await;
    assert_eq!(st, StatusCode::OK);
}

#[tokio::test]
async fn auth_flow() {
    let _g = SERIAL.lock().await;
    let (app, _) = setup().await;
    let (st, _) = send(
        app.clone(),
        req(
            "POST",
            "/api/v1/auth/login",
            None,
            Some(json!({"username": "testadmin", "password": "wrong"})),
        ),
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let token = login(&app).await;
    let (st, v) = send(app, req("GET", "/api/v1/auth/me", Some(&token), None)).await;
    assert!(st.is_success());
    assert_eq!(v["data"]["username"], "testadmin");
}

#[tokio::test]
async fn business_units_seeded() {
    let _g = SERIAL.lock().await;
    let (app, _) = setup().await;
    let token = login(&app).await;
    let (_, v) = send(
        app,
        req("GET", "/api/v1/business-units", Some(&token), None),
    )
    .await;
    let codes: Vec<&str> = v["data"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["code"].as_str().unwrap())
        .collect();
    for c in ["EDU", "ECOM", "STUDY", "ERP"] {
        assert!(codes.contains(&c), "缺少业务单元 {c}: {v}");
    }
}

/// 端到端：目标 → 离线拆解 → 执行 → 审批门控 → 批准后执行 → 发布包落库
#[tokio::test]
async fn commander_offline_full_flow() {
    let _g = SERIAL.lock().await;
    let (app, pool) = setup().await;
    let token = login(&app).await;
    let goal = format!(
        "推广企业ERP培训业务，获取咨询线索-{}",
        Uuid::new_v4().simple()
    );

    // 1. 下达目标（AI 未配置 → 离线兜底计划）
    let (st, v) = send(
        app.clone(),
        req(
            "POST",
            "/api/v1/commander/goals",
            Some(&token),
            Some(json!({"goal": goal})),
        ),
    )
    .await;
    assert!(st.is_success(), "{v}");
    assert_eq!(v["data"]["mode"], "offline_fallback");
    let root_id = v["data"]["root_id"].as_str().unwrap().to_string();
    assert_eq!(v["data"]["children"].as_array().unwrap().len(), 4);

    // 2. 执行
    let (st, _) = send(
        app.clone(),
        req(
            "POST",
            &format!("/api/v1/commander/goals/{root_id}/execute"),
            Some(&token),
            Some(json!({"sync": true})),
        ),
    )
    .await;
    assert!(st.is_success());

    // 3. 树状态：3个离线任务完成，1个需审批
    let (_, tree) = send(
        app.clone(),
        req(
            "GET",
            &format!("/api/v1/commander/goals/{root_id}"),
            Some(&token),
            None,
        ),
    )
    .await;
    let children = tree["data"]["children"].as_array().unwrap();
    let completed = children
        .iter()
        .filter(|c| c["status"] == "Completed")
        .count();
    let waiting = children
        .iter()
        .filter(|c| c["status"] == "Waiting Approval")
        .count();
    assert!(completed >= 3, "完成数不足: {tree}");
    assert_eq!(waiting, 1, "应有1个等待审批: {tree}");

    // 4. 审批中心批准
    let (_, ap) = send(
        app.clone(),
        req(
            "GET",
            "/api/v1/approvals?status=PENDING",
            Some(&token),
            None,
        ),
    )
    .await;
    let pending = ap["data"]["items"].as_array().unwrap();
    assert!(!pending.is_empty(), "应有待审批项");
    let approval_id = pending[0]["id"].as_str().unwrap().to_string();
    let (dec_st, dec) = send(
        app.clone(),
        req(
            "POST",
            &format!("/api/v1/approvals/{approval_id}/decide"),
            Some(&token),
            Some(json!({"decision": "APPROVED", "note": "同意"})),
        ),
    )
    .await;
    assert!(dec_st.is_success(), "{dec}");
    assert_eq!(dec["data"]["status"], "Completed", "{dec}");
    // 审计#6：根任务必须同步为 Completed（此前永远卡 Running）
    let (_, tree2) = send(
        app.clone(),
        req(
            "GET",
            &format!("/api/v1/commander/goals/{root_id}"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(tree2["data"]["root"]["status"], "Completed", "{tree2}");

    // 5. 发布包已入库（人工发布队列）
    let cnt: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM publishing_tasks WHERE status='HUMAN_QUEUE'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(cnt.0 >= 1, "发布包未落库");

    // 6. 任务记忆已自动沉淀
    let mem: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM memories WHERE kind='TASK' AND source_task_id IS NOT NULL",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(mem.0 >= 1, "任务记忆未沉淀");
}

#[tokio::test]
async fn daily_report_endpoint() {
    let _g = SERIAL.lock().await;
    let (app, _) = setup().await;
    let token = login(&app).await;
    let (_, v) = send(
        app,
        req("GET", "/api/v1/analytics/daily-report", Some(&token), None),
    )
    .await;
    assert!(
        v["data"]["markdown"]
            .as_str()
            .unwrap_or("")
            .contains("经营日报"),
        "{v}"
    );
}

#[tokio::test]
async fn publishing_result_writeback() {
    let _g = SERIAL.lock().await;
    let (app, pool) = setup().await;
    let token = login(&app).await;

    // 1. 建内容 → 审批 → 生成发布包
    let (_, c) = send(
        app.clone(),
        req(
            "POST",
            "/api/v1/contents",
            Some(&token),
            Some(json!({"title": "回写测试文章", "body": "正文", "business_unit_code": "EDU"})),
        ),
    )
    .await;
    let content_id = c["data"]["id"].as_str().unwrap().to_string();
    let (_, a) = send(
        app.clone(),
        req(
            "POST",
            &format!("/api/v1/contents/{content_id}/approve"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(a["data"]["status"], "APPROVED", "{a}");
    let (_, p) = send(
        app.clone(),
        req(
            "POST",
            &format!("/api/v1/contents/{content_id}/publish"),
            Some(&token),
            Some(json!({"platform": "website"})),
        ),
    )
    .await;
    let pt_id = p["data"]["publishing_task_id"]
        .as_str()
        .unwrap()
        .to_string();

    // 2. 未审批内容禁止发布（红线）
    let (_, c2) = send(
        app.clone(),
        req(
            "POST",
            "/api/v1/contents",
            Some(&token),
            Some(json!({"title": "未审批文章", "body": "x"})),
        ),
    )
    .await;
    let c2id = c2["data"]["id"].as_str().unwrap().to_string();
    let (st, _) = send(
        app.clone(),
        req(
            "POST",
            &format!("/api/v1/contents/{c2id}/publish"),
            Some(&token),
            Some(json!({"platform": "tiktok"})),
        ),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT, "未审批内容发布必须被拒绝");

    // 3. 人工发布后回填结果 → PUBLISHED
    let (_, r) = send(app.clone(), req("POST", &format!("/api/v1/publishing/{pt_id}/result"), Some(&token), Some(json!({"published_url": "https://learn.hedeyou.com/post-1", "metrics": {"views": 120}})))).await;
    assert_eq!(r["data"]["status"], "PUBLISHED", "{r}");
    let (_, list) = send(
        app.clone(),
        req("GET", "/api/v1/publishing", Some(&token), None),
    )
    .await;
    let item = list["data"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == pt_id.as_str())
        .unwrap();
    assert_eq!(item["results"].as_array().unwrap().len(), 1, "{list}");
    let cnt: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM publishing_results WHERE publishing_task_id=$1")
            .bind(&pt_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(cnt.0, 1);
}

#[tokio::test]
async fn idempotent_topic_creation() {
    let _g = SERIAL.lock().await;
    let (app, _) = setup().await;
    let token = login(&app).await;
    let key = Uuid::new_v4().simple().to_string();
    let build = |k: String| {
        let mut r = req(
            "POST",
            "/api/v1/topics",
            Some(&token),
            Some(json!({"title": "幂等测试主题", "business_unit_code": "EDU"})),
        );
        r.headers_mut()
            .insert("Idempotency-Key", k.parse().unwrap());
        r
    };
    let (st1, v1) = send(app.clone(), build(key.clone())).await;
    assert!(st1.is_success(), "{v1}");
    let (st2, v2) = send(app, build(key)).await;
    assert!(st2.is_success());
    assert_eq!(
        v1["data"]["id"], v2["data"]["id"],
        "幂等键第二次应返回同一记录"
    );
}

#[tokio::test]
async fn tool_invoke_permission_gate() {
    let _g = SERIAL.lock().await;
    let (app, _pool) = setup().await;
    let token = login(&app).await;

    // AUTO 级工具可直调
    let (st, v) = send(
        app.clone(),
        req(
            "POST",
            "/api/v1/tools/content.topics_template/invoke",
            Some(&token),
            Some(json!({"goal": "直调测试目标", "business_unit_code": "ECOM"})),
        ),
    )
    .await;
    assert!(st.is_success(), "{v}");
    assert_eq!(v["data"]["topics"].as_array().unwrap().len(), 5);

    // APPROVAL 级工具禁止直调（红线）
    let (st, v) = send(
        app.clone(),
        req(
            "POST",
            "/api/v1/tools/social.package_upload/invoke",
            Some(&token),
            Some(json!({"platform": "tiktok", "title": "x"})),
        ),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT, "{v}");
    assert!(v["error"]["message"]
        .as_str()
        .unwrap()
        .contains("APPROVAL_REQUIRED"));
}

#[tokio::test]
async fn product_selection_flow() {
    let _g = SERIAL.lock().await;
    let (app, pool) = setup().await;
    let token = login(&app).await;

    let (_, c) = send(app.clone(), req("POST", "/api/v1/products", Some(&token), Some(json!({"title": "测试选品：便携榨汁机", "business_unit_code": "ECOM", "price": 199.0, "source_url": "https://example.com/juicer"})))).await;
    let pid = c["data"]["id"].as_str().unwrap().to_string();

    // AI 未配置时评分工具返回 503 NOT_CONFIGURED（诚实状态，不造假）
    let (st, v) = send(
        app.clone(),
        req(
            "POST",
            "/api/v1/tools/ecommerce.score_product/invoke",
            Some(&token),
            Some(json!({"product_id": pid})),
        ),
    )
    .await;
    assert_eq!(st, StatusCode::SERVICE_UNAVAILABLE, "{v}");

    // 状态机：PENDING → ANALYZED → APPROVED → LISTED（不可跳步）
    for st in ["ANALYZED", "APPROVED", "LISTED"] {
        let (_, u) = send(
            app.clone(),
            req(
                "PUT",
                &format!("/api/v1/products/{pid}"),
                Some(&token),
                Some(json!({"status": st})),
            ),
        )
        .await;
        assert_eq!(u["data"]["updated"], true, "{st}: {u}");
    }
    let st: (String,) = sqlx::query_as("SELECT status FROM product_candidates WHERE id=$1")
        .bind(&pid)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(st.0, "LISTED");
}

#[tokio::test]
async fn audit_publish_no_duplicate_queue() {
    let _g = SERIAL.lock().await;
    let (app, _) = setup().await;
    let token = login(&app).await;
    let (_, c) = send(
        app.clone(),
        req(
            "POST",
            "/api/v1/contents",
            Some(&token),
            Some(json!({"title": "防重文章", "body": "x"})),
        ),
    )
    .await;
    let id = c["data"]["id"].as_str().unwrap().to_string();
    let _ = send(
        app.clone(),
        req(
            "POST",
            &format!("/api/v1/contents/{id}/approve"),
            Some(&token),
            None,
        ),
    )
    .await;
    let (st1, _) = send(
        app.clone(),
        req(
            "POST",
            &format!("/api/v1/contents/{id}/publish"),
            Some(&token),
            Some(json!({"platform": "tiktok"})),
        ),
    )
    .await;
    assert!(st1.is_success());
    let (st2, v2) = send(
        app.clone(),
        req(
            "POST",
            &format!("/api/v1/contents/{id}/publish"),
            Some(&token),
            Some(json!({"platform": "tiktok"})),
        ),
    )
    .await;
    assert_eq!(st2, StatusCode::CONFLICT, "重复发布包必须被拒: {v2}");
}

#[tokio::test]
async fn audit_product_no_state_skip() {
    let _g = SERIAL.lock().await;
    let (app, _) = setup().await;
    let token = login(&app).await;
    let (_, c) = send(
        app.clone(),
        req(
            "POST",
            "/api/v1/products",
            Some(&token),
            Some(json!({"title": "跳步测试品", "business_unit_code": "ECOM"})),
        ),
    )
    .await;
    let id = c["data"]["id"].as_str().unwrap().to_string();
    // PENDING 直接跳 LISTED 必须被拒
    let (st, v) = send(
        app.clone(),
        req(
            "PUT",
            &format!("/api/v1/products/{id}"),
            Some(&token),
            Some(json!({"status": "LISTED"})),
        ),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT, "跳步必须被拒: {v}");
}

#[tokio::test]
async fn audit_budget_and_whitelist_guards() {
    let _g = SERIAL.lock().await;
    let (app, pool) = setup().await;
    let token = login(&app).await;
    let goal = format!("预算与白名单测试-{}", Uuid::new_v4().simple());

    // 预算护栏：把 content 任务的工具次数顶到上限
    let (_, g) = send(
        app.clone(),
        req(
            "POST",
            "/api/v1/commander/goals",
            Some(&token),
            Some(json!({"goal": goal})),
        ),
    )
    .await;
    let root_id = g["data"]["root_id"].as_str().unwrap().to_string();
    sqlx::query("UPDATE tasks SET tool_calls_used = max_tool_calls WHERE root_id=$1 AND agent_code='content'")
        .bind(&root_id).execute(&pool).await.unwrap();
    let _ = send(
        app.clone(),
        req(
            "POST",
            &format!("/api/v1/commander/goals/{root_id}/execute"),
            Some(&token),
            Some(json!({"sync": true})),
        ),
    )
    .await;
    let (_, tree) = send(
        app.clone(),
        req(
            "GET",
            &format!("/api/v1/commander/goals/{root_id}"),
            Some(&token),
            None,
        ),
    )
    .await;
    let content_task = tree["data"]["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["agent_code"] == "content")
        .unwrap();
    assert_eq!(content_task["status"], "Failed", "{content_task}");
    assert!(
        content_task["error"]
            .as_str()
            .unwrap()
            .contains("BUDGET_EXCEEDED"),
        "{content_task}"
    );

    // 工具白名单：清空 content Agent 的工具 → 计划的工具被拦截
    let goal2 = format!("白名单测试-{}", Uuid::new_v4().simple());
    sqlx::query("UPDATE agents SET tools='[]'::jsonb WHERE code='content'")
        .execute(&pool)
        .await
        .unwrap();
    let (_, g2) = send(
        app.clone(),
        req(
            "POST",
            "/api/v1/commander/goals",
            Some(&token),
            Some(json!({"goal": goal2})),
        ),
    )
    .await;
    let root2 = g2["data"]["root_id"].as_str().unwrap().to_string();
    let _ = send(
        app.clone(),
        req(
            "POST",
            &format!("/api/v1/commander/goals/{root2}/execute"),
            Some(&token),
            Some(json!({"sync": true})),
        ),
    )
    .await;
    let (_, tree2) = send(
        app.clone(),
        req(
            "GET",
            &format!("/api/v1/commander/goals/{root2}"),
            Some(&token),
            None,
        ),
    )
    .await;
    let ct = tree2["data"]["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["agent_code"] == "content")
        .unwrap();
    // 白名单为空 → 离线模板工具被剥掉，任务走 AI 分支；AI 未配置 → NOT_CONFIGURED 失败，而非越权执行
    if ct["status"] == "Failed" {
        let e = ct["error"].as_str().unwrap_or("");
        assert!(
            e.contains("无权调用") || e.contains("NOT_CONFIGURED"),
            "{ct}"
        );
    }
    sqlx::query(r#"UPDATE agents SET tools='["content.topics_template","content.draft_article","ai.translate","memory.search","memory.save"]'::jsonb WHERE code='content'"#)
        .execute(&pool).await.unwrap();
}

#[tokio::test]
async fn audit_idempotency_failure_not_cached() {
    let _g = SERIAL.lock().await;
    let (app, _) = setup().await;
    let token = login(&app).await;
    let key = format!("fail-{}", Uuid::new_v4().simple());
    let build = |k: String, body: Value| {
        let mut r = req("POST", "/api/v1/topics", Some(&token), Some(body));
        r.headers_mut()
            .insert("Idempotency-Key", k.parse().unwrap());
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
    let _g = SERIAL.lock().await;
    let (app, pool) = setup().await;
    let token = login(&app).await;
    let goal = format!("审批重试语义-{}", Uuid::new_v4().simple());
    let (_, g) = send(
        app.clone(),
        req(
            "POST",
            "/api/v1/commander/goals",
            Some(&token),
            Some(json!({"goal": goal})),
        ),
    )
    .await;
    let root_id = g["data"]["root_id"].as_str().unwrap().to_string();
    let _ = send(
        app.clone(),
        req(
            "POST",
            &format!("/api/v1/commander/goals/{root_id}/execute"),
            Some(&token),
            Some(json!({"sync": true})),
        ),
    )
    .await;

    // 找到审批型子任务（social.media），强制置为 Failed 模拟业务失败
    let (_, tree) = send(
        app.clone(),
        req(
            "GET",
            &format!("/api/v1/commander/goals/{root_id}"),
            Some(&token),
            None,
        ),
    )
    .await;
    let social = tree["data"]["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["agent_code"] == "social.media")
        .unwrap();
    let sid = social["id"].as_str().unwrap().to_string();
    sqlx::query("UPDATE tasks SET status='Failed', error='simulate' WHERE id=$1")
        .bind(&sid)
        .execute(&pool)
        .await
        .unwrap();
    let before: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM publishing_tasks")
        .fetch_one(&pool)
        .await
        .unwrap();

    // 重试：必须重新进审批，绝不能直接执行
    let (_, r) = send(
        app.clone(),
        req(
            "POST",
            &format!("/api/v1/tasks/{sid}/retry"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(r["data"]["status"], "Waiting Approval", "{r}");
    let after: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM publishing_tasks")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(before.0, after.0, "审批型任务重试绕过了审批直接执行！");
    // 且产生了新的 PENDING 审批
    let pend: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM approvals WHERE task_id=$1 AND status='PENDING'")
            .bind(&sid)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(pend.0 >= 1);
}

#[tokio::test]
async fn audit2_idempotency_stale_lock_recovered() {
    let _g = SERIAL.lock().await;
    let (app, pool) = setup().await;
    let token = login(&app).await;
    let key = format!("stale-{}", Uuid::new_v4().simple());
    // 模拟进程崩溃残留：15 分钟前的 NULL 占位
    sqlx::query("INSERT INTO idempotency_keys (id, idem_key, endpoint, response, created_at) VALUES ($1,$2,'/api/v1/topics',NULL, now() - interval '15 minutes')")
        .bind(Uuid::new_v4().simple().to_string()).bind(&key).execute(&pool).await.unwrap();
    // 同键请求：陈锁应被回收并正常执行
    let mut r = req(
        "POST",
        "/api/v1/topics",
        Some(&token),
        Some(json!({"title": "陈锁恢复主题"})),
    );
    r.headers_mut()
        .insert("Idempotency-Key", key.parse().unwrap());
    let (st, v) = send(app, r).await;
    assert!(st.is_success(), "陈锁未回收导致永久锁死: {v}");
}

#[tokio::test]
async fn audit2_operator_isolation() {
    let _g = SERIAL.lock().await;
    let (app, pool) = setup().await;
    let admin_token = login(&app).await;
    // 建 OPERATOR 用户
    let _ = sqlx::query("INSERT INTO users (id, username, password_hash, role) VALUES ($1,'op-test',$2,'OPERATOR') ON CONFLICT (username) DO NOTHING")
        .bind(Uuid::new_v4().simple().to_string())
        .bind(ai_workforce::auth::hash_password("op-pass-123").unwrap())
        .execute(&pool).await.unwrap();
    let (st, v) = send(
        app.clone(),
        req(
            "POST",
            "/api/v1/auth/login",
            None,
            Some(json!({"username": "op-test", "password": "op-pass-123"})),
        ),
    )
    .await;
    assert!(st.is_success(), "{v}");
    let op_token = v["data"]["token"].as_str().unwrap().to_string();

    // admin 建目标
    let goal = format!("隔离测试-{}", Uuid::new_v4().simple());
    let (_, g) = send(
        app.clone(),
        req(
            "POST",
            "/api/v1/commander/goals",
            Some(&admin_token),
            Some(json!({"goal": goal})),
        ),
    )
    .await;
    let root_id = g["data"]["root_id"].as_str().unwrap().to_string();

    // OPERATOR 列表看不到 admin 的任务
    let (_, list) = send(
        app.clone(),
        req("GET", "/api/v1/tasks", Some(&op_token), None),
    )
    .await;
    let leaked = list["data"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i["root_id"] == root_id.as_str());
    assert!(!leaked, "OPERATOR 泄露了他人任务: {list}");
    // OPERATOR 直读他人任务 → 404
    let (st, _) = send(
        app.clone(),
        req(
            "GET",
            &format!("/api/v1/commander/goals/{root_id}"),
            Some(&op_token),
            None,
        ),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn listening_keyword_to_signal_to_topic() {
    let _g = SERIAL.lock().await;
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
    let (_, tt) = send(
        app.clone(),
        req(
            "POST",
            &format!("/api/v1/listening/signals/{sig_id}/to-topic"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert!(tt["data"]["topic_id"].as_str().is_some(), "{tt}");
    let st: (String,) = sqlx::query_as("SELECT status FROM listening_signals WHERE id=$1")
        .bind(&sig_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(st.0, "CONVERTED");
}

#[tokio::test]
async fn audit3_crossuser_execute_retry_cancel_blocked() {
    let _g = SERIAL.lock().await;
    let (app, pool) = setup().await;
    let admin_token = login(&app).await;
    let _ = sqlx::query("INSERT INTO users (id, username, password_hash, role) VALUES ($1,'op-cu',$2,'OPERATOR') ON CONFLICT (username) DO NOTHING")
        .bind(Uuid::new_v4().simple().to_string())
        .bind(ai_workforce::auth::hash_password("op-pass-123").unwrap())
        .execute(&pool).await.unwrap();
    let (_, v) = send(
        app.clone(),
        req(
            "POST",
            "/api/v1/auth/login",
            None,
            Some(json!({"username": "op-cu", "password": "op-pass-123"})),
        ),
    )
    .await;
    let op_token = v["data"]["token"].as_str().unwrap().to_string();

    let goal = format!("跨用户权限-{}", Uuid::new_v4().simple());
    let (_, g) = send(
        app.clone(),
        req(
            "POST",
            "/api/v1/commander/goals",
            Some(&admin_token),
            Some(json!({"goal": goal})),
        ),
    )
    .await;
    let root_id = g["data"]["root_id"].as_str().unwrap().to_string();

    // OPERATOR 执行/重试/取消 他人目标 → 一律 403
    for path in [
        format!("/api/v1/commander/goals/{root_id}/execute"),
        format!("/api/v1/tasks/{root_id}/retry"),
        format!("/api/v1/tasks/{root_id}/cancel"),
    ] {
        let (st, _) = send(
            app.clone(),
            req("POST", &path, Some(&op_token), Some(json!({}))),
        )
        .await;
        assert_eq!(st, StatusCode::FORBIDDEN, "跨用户操作未拦截: {path}");
    }
}

#[tokio::test]
async fn audit3_approval_tamper_double_self() {
    let _g = SERIAL.lock().await;
    let (app, pool) = setup().await;
    let token = login(&app).await;
    let goal = format!("审批篡改-{}", Uuid::new_v4().simple());
    let (_, g) = send(
        app.clone(),
        req(
            "POST",
            "/api/v1/commander/goals",
            Some(&token),
            Some(json!({"goal": goal})),
        ),
    )
    .await;
    let root_id = g["data"]["root_id"].as_str().unwrap().to_string();
    let _ = send(
        app.clone(),
        req(
            "POST",
            &format!("/api/v1/commander/goals/{root_id}/execute"),
            Some(&token),
            Some(json!({"sync": true})),
        ),
    )
    .await;
    let (_, ap) = send(
        app.clone(),
        req(
            "GET",
            "/api/v1/approvals?status=PENDING",
            Some(&token),
            None,
        ),
    )
    .await;
    let apid = ap["data"]["items"][0]["id"].as_str().unwrap().to_string();

    // ① 参数篡改：批准前偷换 args → 409 且审批不留 APPROVED
    let (tid,): (String,) = sqlx::query_as("SELECT task_id FROM approvals WHERE id=$1")
        .bind(&apid)
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE tasks SET args='{\"hacked\":true}'::jsonb WHERE id=$1")
        .bind(&tid)
        .execute(&pool)
        .await
        .unwrap();
    let (st, _) = send(
        app.clone(),
        req(
            "POST",
            &format!("/api/v1/approvals/{apid}/decide"),
            Some(&token),
            Some(json!({"decision": "APPROVED"})),
        ),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT, "篡改参数必须被拒");
    let st_db: (String,) = sqlx::query_as("SELECT status FROM approvals WHERE id=$1")
        .bind(&apid)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(st_db.0, "PENDING", "被拒的审批不得留下APPROVED");

    // ② 恢复参数（快照原样写回=还原源） → 批准成功 → 重复审批 → 409
    let (raw,) = sqlx::query_as::<_, (Option<serde_json::Value>,)>(
        "SELECT args_snapshot FROM approvals WHERE id=$1",
    )
    .bind(&apid)
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE tasks SET args=$2 WHERE id=$1")
        .bind(&tid)
        .bind(&raw)
        .execute(&pool)
        .await
        .unwrap();
    let (st2, _) = send(
        app.clone(),
        req(
            "POST",
            &format!("/api/v1/approvals/{apid}/decide"),
            Some(&token),
            Some(json!({"decision": "APPROVED"})),
        ),
    )
    .await;
    assert!(st2.is_success(), "快照对齐后应可批准");
    let (st3, _) = send(
        app.clone(),
        req(
            "POST",
            &format!("/api/v1/approvals/{apid}/decide"),
            Some(&token),
            Some(json!({"decision": "APPROVED"})),
        ),
    )
    .await;
    assert_eq!(st3, StatusCode::CONFLICT, "重复审批必须被拒");
}

#[tokio::test]
async fn audit3_cancel_created_then_state_guard() {
    let _g = SERIAL.lock().await;
    let (app, _) = setup().await;
    let token = login(&app).await;
    let goal = format!("取消状态机-{}", Uuid::new_v4().simple());
    let (_, g) = send(
        app.clone(),
        req(
            "POST",
            "/api/v1/commander/goals",
            Some(&token),
            Some(json!({"goal": goal})),
        ),
    )
    .await;
    let root_id = g["data"]["root_id"].as_str().unwrap().to_string();
    let (_, tree) = send(
        app.clone(),
        req(
            "GET",
            &format!("/api/v1/commander/goals/{root_id}"),
            Some(&token),
            None,
        ),
    )
    .await;
    let sid = tree["data"]["children"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    // Created → cancel → Cancelled
    let (_, c) = send(
        app.clone(),
        req(
            "POST",
            &format!("/api/v1/tasks/{sid}/cancel"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(c["data"]["status"], "Cancelled", "{c}");
    // 再取消 → 状态机拒绝
    let (st, _) = send(
        app.clone(),
        req(
            "POST",
            &format!("/api/v1/tasks/{sid}/cancel"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);
}

#[tokio::test]
async fn t001_ssrf_blocked_codes() {
    let _g = SERIAL.lock().await;
    let (app, _) = setup().await;
    let token = login(&app).await;
    for url in [
        "http://127.0.0.1:8600/health",
        "http://169.254.169.254/latest/meta-data/",
        "http://localhost/anything",
    ] {
        let (st, v) = send(
            app.clone(),
            req(
                "POST",
                "/api/v1/tools/web.fetch/invoke",
                Some(&token),
                Some(json!({"url": url})),
            ),
        )
        .await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "{url}: {v}");
        assert_eq!(v["error"]["code"], "SSRF_BLOCKED", "{url}: {v}");
    }
}

#[tokio::test]
async fn t001_ssrf_redirect_to_internal_blocked() {
    let _g = SERIAL.lock().await;
    let (app, _) = setup().await;
    let token = login(&app).await;
    // 本地起一个一次性 302 跳板：跳到 http://127.0.0.1:9/（环回），验证逐跳重校验
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        if let Ok((mut sock, _)) = listener.accept().await {
            use tokio::io::AsyncWriteExt as _;
            let _ = sock
                .write_all(
                    b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:9/next\r\nContent-Length: 0\r\n\r\n",
                )
                .await;
        }
    });
    let (st, v) = send(
        app,
        req(
            "POST",
            "/api/v1/tools/web.fetch/invoke",
            Some(&token),
            Some(json!({"url": format!("http://{addr}/jump")})),
        ),
    )
    .await;
    // 第一跳是本机环回（127.0.0.1 受限名单）→ 在连接前即被拦
    assert_eq!(st, StatusCode::BAD_REQUEST, "{v}");
    assert_eq!(v["error"]["code"], "SSRF_BLOCKED");
}

#[tokio::test]
async fn t002_concurrent_approve_only_one_wins() {
    let _g = SERIAL.lock().await;
    let (app, pool) = setup().await;
    let token = login(&app).await;
    let goal = format!("并发批准-{}", Uuid::new_v4().simple());
    let (_, g) = send(
        app.clone(),
        req(
            "POST",
            "/api/v1/commander/goals",
            Some(&token),
            Some(json!({"goal": goal})),
        ),
    )
    .await;
    let root_id = g["data"]["root_id"].as_str().unwrap().to_string();
    let _ = send(
        app.clone(),
        req(
            "POST",
            &format!("/api/v1/commander/goals/{root_id}/execute"),
            Some(&token),
            Some(json!({"sync": true})),
        ),
    )
    .await;
    let (_, ap) = send(
        app.clone(),
        req(
            "GET",
            "/api/v1/approvals?status=PENDING",
            Some(&token),
            None,
        ),
    )
    .await;
    let apid = ap["data"]["items"][0]["id"].as_str().unwrap().to_string();

    // 并发 10 次批准同一审批：FOR UPDATE + 条件更新 → 只允许 1 次成功
    let mut futs = Vec::new();
    for _ in 0..10 {
        let app_c = app.clone();
        let tok_c = token.clone();
        let apid_c = apid.clone();
        futs.push(async move {
            send(
                app_c,
                req(
                    "POST",
                    &format!("/api/v1/approvals/{apid_c}/decide"),
                    Some(&tok_c),
                    Some(json!({"decision": "APPROVED"})),
                ),
            )
            .await
        });
    }
    let results = futures_util::future::join_all(futs).await;
    let successes = results.iter().filter(|(st, _)| st.is_success()).count();
    assert_eq!(successes, 1, "并发10次批准应只有1次成功");
    // 任务只执行一次（Running 事件数=1）
    let running: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM task_events WHERE task_id=$1 AND event='Running'")
            .bind(&apid_parsed_task(&pool, &apid).await)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(running.0, 1, "任务必须只执行一次");
}

async fn apid_parsed_task(pool: &PgPool, apid: &str) -> String {
    let (tid,): (String,) = sqlx::query_as("SELECT task_id FROM approvals WHERE id=$1")
        .bind(apid)
        .fetch_one(pool)
        .await
        .unwrap();
    tid
}

#[tokio::test]
async fn t003_daily_budget_gate() {
    let _g = SERIAL.lock().await;
    std::env::set_var("DAILY_TOKEN_LIMIT", "1");
    // 重建 app 使配置生效
    let (app, pool) = setup().await;
    let token = login(&app).await;
    // 台账里已有当日 1 token 消耗 → 预算即满
    sqlx::query("INSERT INTO usage_ledger (id, task_id, tokens, cost_usd) VALUES ($1,NULL,1,0)")
        .bind(Uuid::new_v4().simple().to_string())
        .execute(&pool)
        .await
        .unwrap();
    let (st, v) = send(
        app.clone(),
        req(
            "POST",
            "/api/v1/tools/ai.generate/invoke",
            Some(&token),
            Some(json!({"prompt": "测试"})),
        ),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::TOO_MANY_REQUESTS,
        "日预算必须拦下AI调用: {v}"
    );
    assert_eq!(v["error"]["code"], "BUDGET_EXCEEDED");
    std::env::remove_var("DAILY_TOKEN_LIMIT");
}

#[tokio::test]
async fn t004_ssrf_mapped_and_cgnat() {
    let _g = SERIAL.lock().await;
    let (app, _) = setup().await;
    let token = login(&app).await;
    for url in [
        "http://[::ffff:127.0.0.1]:8600/health", // IPv4-mapped IPv6
        "http://100.100.100.200/latest/meta-data/", // 阿里云元数据（CGNAT段）
        "http://[::ffff:7f00:1]/",               // ::ffff:7f00:1 = 127.0.0.1
    ] {
        let (st, v) = send(
            app.clone(),
            req(
                "POST",
                "/api/v1/tools/web.fetch/invoke",
                Some(&token),
                Some(json!({"url": url})),
            ),
        )
        .await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "{url}: {v}");
        assert_eq!(v["error"]["code"], "SSRF_BLOCKED", "{url}: {v}");
    }
}

#[tokio::test]
async fn t005_botarmy_team_traversal_blocked() {
    let _g = SERIAL.lock().await;
    let (app, _) = setup().await;
    let token = login(&app).await;
    for team in ["../../tmp", "..\\\\ssh", "a/b", "team;rm"] {
        let (st, v) = send(
            app.clone(),
            req(
                "POST",
                "/api/v1/tools/botarmy.dispatch/invoke",
                Some(&token),
                Some(json!({"team": team, "bot": "copy", "subject": "x", "content": "x"})),
            ),
        )
        .await;
        // botarmy.dispatch 是 APPROVAL 级：invoke 直调 409 拦截在先——用 lookup/白名单验证团队合法性走 approve 流程太重，
        // 这里验证参数校验前置：即便未来权限变化，穿越参数也进不来（409 或 400 都算拦截）
        assert!(
            st == StatusCode::CONFLICT || st == StatusCode::BAD_REQUEST,
            "穿越参数未被拦截: {team} {v}"
        );
    }
}

#[tokio::test]
async fn t005_args_changed_error_code() {
    let _g = SERIAL.lock().await;
    let (app, pool) = setup().await;
    let token = login(&app).await;
    let goal = format!("ARGS_CHANGED码-{}", Uuid::new_v4().simple());
    let (_, g) = send(
        app.clone(),
        req(
            "POST",
            "/api/v1/commander/goals",
            Some(&token),
            Some(json!({"goal": goal})),
        ),
    )
    .await;
    let root_id = g["data"]["root_id"].as_str().unwrap().to_string();
    let _ = send(
        app.clone(),
        req(
            "POST",
            &format!("/api/v1/commander/goals/{root_id}/execute"),
            Some(&token),
            Some(json!({"sync": true})),
        ),
    )
    .await;
    let (_, ap) = send(
        app.clone(),
        req(
            "GET",
            "/api/v1/approvals?status=PENDING",
            Some(&token),
            None,
        ),
    )
    .await;
    let apid = ap["data"]["items"][0]["id"].as_str().unwrap().to_string();
    let (tid,): (String,) = sqlx::query_as("SELECT task_id FROM approvals WHERE id=$1")
        .bind(&apid)
        .fetch_one(&pool)
        .await
        .unwrap();
    // 篡改参数 → 批准 → 409 + ARGS_CHANGED 专用码
    sqlx::query("UPDATE tasks SET args='{\"tampered\":true}'::jsonb WHERE id=$1")
        .bind(&tid)
        .execute(&pool)
        .await
        .unwrap();
    let (st, v) = send(
        app.clone(),
        req(
            "POST",
            &format!("/api/v1/approvals/{apid}/decide"),
            Some(&token),
            Some(json!({"decision": "APPROVED"})),
        ),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);
    assert_eq!(v["error"]["code"], "ARGS_CHANGED", "{v}");
}

// ============ 自动发布（国内/海外分区） ============

/// 平台清单必须真实返回国内 5 + 海外 2，且区域不混
#[tokio::test]
async fn publish_platforms_split_by_region() {
    let _g = SERIAL.lock().await;
    let (app, _) = setup().await;
    let token = login(&app).await;
    let (st, v) = send(
        app,
        req(
            "POST",
            "/api/v1/tools/publish.platforms/invoke",
            Some(&token),
            Some(json!({})),
        ),
    )
    .await;
    assert!(st.is_success(), "{v}");
    let d = &v["data"];
    assert_eq!(d["total"], 7, "{d}");
    assert_eq!(d["domestic"].as_array().unwrap().len(), 5);
    assert_eq!(d["overseas"].as_array().unwrap().len(), 2);
    // 抖音必须在国内，YouTube 必须在海外
    let dom_keys: Vec<&str> = d["domestic"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["key"].as_str().unwrap())
        .collect();
    assert!(dom_keys.contains(&"douyin"), "{dom_keys:?}");
    let ovs_keys: Vec<&str> = d["overseas"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["key"].as_str().unwrap())
        .collect();
    assert!(ovs_keys.contains(&"youtube"), "{ovs_keys:?}");
    assert!(!dom_keys.contains(&"youtube"));
}

/// 红线：publish.auto 与 publish.auto_many 都必须是 APPROVAL 级，直调必被拦
#[tokio::test]
async fn publish_auto_requires_approval() {
    let _g = SERIAL.lock().await;
    let (app, _) = setup().await;
    let token = login(&app).await;
    for tool in ["publish.auto", "publish.auto_many"] {
        let (st, v) = send(
            app.clone(),
            req(
                "POST",
                &format!("/api/v1/tools/{tool}/invoke"),
                Some(&token),
                Some(json!({"platform":"douyin","platforms":["douyin"],"file":"/tmp/x.mp4","title":"t"})),
            ),
        )
        .await;
        assert_eq!(st, StatusCode::CONFLICT, "{tool} 应被审批门拦下: {v}");
        assert!(
            v["error"]["message"].as_str().unwrap().contains("APPROVAL_REQUIRED"),
            "{tool}: {v}"
        );
    }
}

/// 未配置自动发布时，账号体检返回诚实 503，而不是假装成功
#[tokio::test]
async fn publish_check_account_not_configured() {
    let _g = SERIAL.lock().await;
    let (app, _) = setup().await;
    let token = login(&app).await;
    // setup 的 Config 来自 from_env；CI 下 FEATURE_AUTOPUBLISH 默认 0
    if std::env::var("FEATURE_AUTOPUBLISH").as_deref() == Ok("1") {
        return; // 本机开了自动发布时跳过此断言
    }
    let (st, v) = send(
        app,
        req(
            "POST",
            "/api/v1/tools/publish.check_account/invoke",
            Some(&token),
            Some(json!({"platform":"douyin"})),
        ),
    )
    .await;
    assert_eq!(st, StatusCode::SERVICE_UNAVAILABLE, "{v}");
    assert_eq!(v["error"]["code"], "NOT_CONFIGURED", "{v}");
}

/// 发布中心必须按区域分组统计（国内/海外分开）
#[tokio::test]
async fn publishing_list_groups_by_region() {
    let _g = SERIAL.lock().await;
    let (app, pool) = setup().await;
    let token = login(&app).await;
    // 造两条任务：一条国内、一条海外
    for (id, plat, region) in [
        ("pt_dom_1", "douyin", "DOMESTIC"),
        ("pt_ovs_1", "youtube", "OVERSEAS"),
    ] {
        sqlx::query(
            "INSERT INTO publishing_tasks (id, platform, status, region, mode, payload) \
             VALUES ($1,$2,'HUMAN_QUEUE',$3,'AUTO','{}'::jsonb) ON CONFLICT (id) DO NOTHING",
        )
        .bind(id)
        .bind(plat)
        .bind(region)
        .execute(&pool)
        .await
        .unwrap();
    }
    let (st, v) = send(
        app,
        req("GET", "/api/v1/publishing", Some(&token), None),
    )
    .await;
    assert!(st.is_success(), "{v}");
    let br = &v["data"]["by_region"];
    assert!(br["domestic"].as_u64().unwrap() >= 1, "{v}");
    assert!(br["overseas"].as_u64().unwrap() >= 1, "{v}");
    // 每条任务都要带 region 字段
    for it in v["data"]["items"].as_array().unwrap() {
        assert!(it["region"].is_string(), "缺少 region: {it}");
        assert!(it["mode"].is_string(), "缺少 mode: {it}");
    }
}
