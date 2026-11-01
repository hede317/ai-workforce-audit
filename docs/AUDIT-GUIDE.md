# AI Workforce 审计指南（给外部 Agent / 审阅工程师）

本仓是 HedeYou AI Workforce 的审计镜像。生产主仓为私有仓（同 commit）。

## 项目一句话

AI Agent 工作平台：自然语言经营目标 → CEO Agent 拆解 → 11 名专业虚拟员工调用注册工具执行 → 敏感动作强制人工审批 → 四类记忆沉淀 → 数据分析日报。Rust 单二进制 + PostgreSQL 16 (pgvector) + Next.js。

## 核心文件地图

| 文件 | 内容 | 审计价值 |
|---|---|---|
| `backend/src/orchestrator.rs` | 任务编排器：目标拆解（AI优先/离线兜底）、状态机、预算护栏、审批门控、记忆钩子 | **最核心**。看状态流转是否有漏洞、预算是否可被绕过 |
| `backend/src/tools.rs` | Tool Registry：15 个工具 + AUTO/APPROVAL 权限分级 + 实现 | 看权限门是否一致、外部调用失败路径 |
| `backend/src/ai.rs` | OpenAI 兼容网关适配 + embedding + 余弦 | 协议兼容性 |
| `backend/src/auth.rs` | Argon2 + JWT + 提取器 | 认证安全 |
| `backend/src/routers/` | 全部 REST 端点（统一响应封套/审计/幂等中间件在 mod.rs） | 越权/注入面 |
| `backend/migrations/` | 3 个迁移：17 张表 + pgvector | Schema 设计 |
| `backend/tests/api.rs` | 9 条集成测试（离线全链路） | 测试是否测到点子上 |
| `frontend/` | Next.js 暗色控制台 9 页 | 前端健壮性 |
| `docs/PRD-v2.0-AI-Workforce.md` | 主规范（架构/红线/验收标准） | 对照实现与规范的偏差 |

## 设计红线（代码级强制，审计请验证）

1. APPROVAL 级工具未获批不得执行——`tools.rs` dispatch 门 + `routers/agents.rs::invoke_tool` 直调拦截（APPROVAL 级直调 → 409）
2. 内容未 APPROVED 不得发布——`routers/business.rs::publish_content`
3. 企业记忆 Agent 只能写 PENDING，用户确认才生效——`tools.rs::memory_save`
4. 商品上架仅 ADMIN——`routers/business.rs::update_product`
5. 幂等：`Idempotency-Key` 中间件（`routers/mod.rs`）

## 怎么跑起来

```bash
# 数据库（需要 PG16，容器名/端口自定）
docker run -d --name pg -e POSTGRES_USER=ai_workforce -e POSTGRES_PASSWORD=ai_workforce -e POSTGRES_DB=ai_workforce -p 5434:5432 pgvector/pgvector:pg16
# 测试（自动建 ai_workforce_test 库）
cd backend && cargo test
# 启动
cargo run    # :8600
cd frontend && npm install && npm run build && npm run start   # :8610
# 登录 admin / change-me-admin（.env 可改，默认值仅为开发便利）
```

无任何 API key 也能全链路跑（离线兜底模式是刻意设计：确定性工具真实工作，AI 依赖项返回 503 NOT_CONFIGURED 而非造假数据）。

## 我们已知的设计取舍（不必重复报告）

- 发布默认走 HUMAN_QUEUE 人工队列（平台官方 API 未接入，合规优先）
- memory 检索：无 EMBED_* 时回落 LIKE；向量检索小规模暴力余弦（<500 条）
- SQLite 不支持：测试直连真实 PG
- 前端自绘组件替代 shadcn（同设计语言，零额外依赖）
- 单业务进程内同步执行子任务（无消息队列；Redis 预留未接）

## 高价值审计问题（欢迎直接提 PR/Issue）

1. 编排器预算护栏（tokens/工具次数/时长/深度）是否有绕过路径？
2. SQL 全部走 sqlx 参数绑定——有没有漏网的字符串拼接？
3. JWT 24h 无刷新令牌——对多日任务是否够用？怎么做最小改动？
4. 幂等中间件存储原始响应体——大响应/竞态有没有问题？
5. n8n 转发端点的鉴权与 SSRF 面？
6. 多租户：tenant_id 已预留但未启用——何时启用、怎么切？

联系方式：通过仓库 Issue 或仓库所有者。

---

## 三审整改记录（2026-11-01，branch audit3-remediation，PR#1）

| 三审发现 | 处置 | 代码位置 |
|---|---|---|
| 跨用户执行/重试/取消 | load_task_for_user 统一所有权闸门 | routers/agents.rs |
| 审批无事务/可重复/可自批 | FOR UPDATE 行锁+PENDING条件更新+事务内快照哈希校验+创建者不自批(ADMIN例外) | orchestrator.rs::decide_approval |
| 取消不能停运行中任务 | CancelRequested 两阶段协作取消+完成守卫(不覆盖取消)+副作用照实存档 | agents.rs/orchestrator.rs |
| DNS重绑定 | 校验后按已验证IP现建钉死客户端(resolve) | tools.rs::fetch_limited |
| 邮件无授权闸门 | 退订/拒绝名单命中即拒，审批+审计保留 | tools.rs::email_send |
| 生产配置 | 弱密钥拒启、compose密码环境变量化、n8n钉版本+本机绑定、.gitignore扩面 | config.rs/docker-compose.yml |
| CI | GitHub Actions: fmt/clippy(-D warnings)/test(pg服务)/前端build/密钥扫描 | .github/workflows/ci.yml |

审计意见驳回记录：globals.css "8 px"（实为 8px，系旧版本误读）。

---

## 四审任务整改记录（T-001~T-003，branch audit4-hardening，24/24绿）

| 任务 | 处置 | 位置 |
|---|---|---|
| T-001 SSRF | 错误码统一400+SSRF_BLOCKED；fetch_one_hop单跳(全IP校验+resolve钉IP+10s)+手动跟随重定向≤3跳逐跳重校验；2MB上限；sitemap入守卫 | tools.rs::fetch_one_hop/fetch_limited |
| T-002 原子化 | 抢单清单(Created/Assigned)对齐；decide事务内置Assigned、提交后执行；FOR UPDATE+条件更新在位 | orchestrator.rs |
| T-003 预算 | usage_ledger(0007)+call_ai前置日预算闸门(429 BUDGET_EXCEEDED)+/api/v1/usage/today+SUM(bigint)::bigint cast | tools.rs/routers/data.rs/migrations/0007 |

新增测试4条：SSRF直击×3(127.0.0.1/169.254.169.254/localhost)、302跳板重定向拦截、并发10批准仅1胜(Running事件=1)、日预算闸门(429)。
