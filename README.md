# HedeYou AI Workforce

> **AI 企业虚拟员工与全域经营自动化平台**——让一个人通过 AI 管理一支虚拟员工团队。
> 依据《HedeYou AI Workforce V2.0 完整产品需求、系统架构与开发实施规范》（docs/PRD-v2.0-AI-Workforce.md）建设。
> V1.0（docs/PRD-v1.0-完整技术开发实施规范.md）降级为参考资料：内容中心数据模型、工作流节点说明仍可用。

## 一句话

用户向 **AI Commander** 下达自然语言经营目标 → CEO Agent 拆解任务 → 分派 11 名专业虚拟员工（Agent）调用注册工具执行 → 需要对外动手的挂**审批中心** → 结果沉淀进**四类记忆** → 数据分析 Agent 出经营日报。

## 技术栈（含规范偏离记录）

| 层 | 规范原定 | 实际采用 | 原因 |
|---|---|---|---|
| 后端 | Python FastAPI + LangGraph | **Rust (Axum + SQLx + Tokio)，单二进制** | 用户 2026-10-31 裁决；单二进制交付=代运营客户部署卖点；Agent Runtime 本就是显式状态机，不依赖 LangGraph |
| 数据库 | PostgreSQL + pgvector | **PostgreSQL 16 + pgvector 就绪** | 用户先改 MySQL 后经对比裁决回归 PG：向量检索（pgvector）、JSONB+GIN、多租户 RLS（复用 deyou-pg 验证过的经验） |
| 前端 | Next.js + Tailwind + shadcn | Next.js 14 + Tailwind + 自绘组件（同设计语言） | shadcn 以等价组件替代，保持零额外依赖 |
| 自动化 | n8n 保留 | n8n 作为确定性调度执行层（WF08 日报模板已备） | 规范一致：n8n 不再是智能决策中心 |
| 部署 | Docker Compose | docker-compose.yml（db/backend/n8n/frontend 四服务） | 一致 |

## 目录

```
ai-workforce/
├── backend/            Rust 单二进制服务（:8600）
│   ├── src/            orchestrator(编排器) / tools(工具注册表) / seed(12 Agent) / routers
│   ├── migrations/     PostgreSQL 建表
│   └── tests/          6 条集成测试（离线全链路）
├── frontend/           Next.js 管理后台（:8610，暗色控制台风格）
├── n8n/workflows/      WF08 每日经营日报（n8n 定时 → 后端 API）
├── docs/               PRD v2.0（主规范）+ PRD v1.0（参考）
└── docker-compose.yml  db(pgvector:5434) + backend(8600) + n8n(3210) + frontend(8610)
```

## 快速启动

```bash
# 1. 数据库（首次）
docker compose up -d db
# 2. 后端（本机开发直跑单二进制）
cd backend && cargo run          # 或 cargo build --release 后跑二进制
# 3. 前端
cd frontend && npm install && npm run dev   # http://localhost:8610
# 4. 测试
cd backend && cargo test
```

管理员账号：`.env` 的 `AWF_ADMIN_USERNAME/PASSWORD`（默认 admin / change-me-admin，**上线必改**）。

要全自动化（AI 拆解/AI 生成内容），在 `.env` 配 `AI_BASE_URL/AI_API_KEY/AI_MODEL`（任意 OpenAI 兼容网关，例如本机 one-api `http://127.0.0.1:3100/v1`）。**未配置 AI 时系统照常可用**：CEO 走离线模板拆解，确定性工具（选题模板/记忆/日报/审批）全部真实工作。

## 已实现 vs 待配置 vs 未做（诚实状态矩阵）

### ✅ 已实现并测试（cargo test 7/7 绿）
- AI Commander：自然语言目标 → 任务拆解（AI 优先 / 离线模板兜底）→ 分派
- Agent Runtime：显式状态机（Created→Planning→Waiting Approval→Assigned→Running→Reviewing→Completed/Failed/Paused/Cancelled）+ 预算护栏（工具次数/Token/时长/深度）
- 12 Agent 注册表（CEO+市场/内容/视频/SEO/社媒/销售/邮件/电商/教育/企业服务/数据分析），提示词可在后台编辑
- Tool Registry：14 个注册工具（AI生成/翻译、离线选题、文章起草、网页抓取、SEO检查、占位视频、MPT出片转发、社媒发布包、WordPress草稿、记忆读写、线索、日报）；权限分级 AUTO/APPROVAL
- 审批中心：红线清单强制审批（发布/邮件/联系客户/改价/广告）；批准后带授权执行、拒绝即取消；全程审计
- 四类记忆 + Memory Manager：任务记忆自动沉淀；企业记忆 Agent 只能写 PENDING，用户确认才生效
- 内容/线索/活动数据模型与 API；发布默认进 HUMAN_QUEUE（不押注任何平台自动发布）
- 幂等（Idempotency-Key）、审计日志、统一响应封套、JWT 三角色（ADMIN/EDITOR/OPERATOR 预留）
- 经营日报（数据真实统计）；n8n 转发接口
- **Sprint 2 增补（同日）**：发布结果回写闭环（publishing_results + 回填 API + 前端发布中心页）；pgvector 向量记忆（EMBED_* 配置后语义检索，未配置回落 LIKE）；SMTP 邮件工具（email.send_draft，APPROVAL 级）

### ⚙️ 代码就绪、需配置凭据（按规范"真实待配置状态"处理，不造假）
- AI 生成/翻译/文章起草 → 配 AI_BASE_URL 等三个变量
- WordPress 发布 → 业务单元 configuration.wp 填 base_url/username/app_password
- MPT 出片 → MPT_WORKER_URL 指向现有 AIOS worker
- n8n 日报定时 → 导入 n8n/workflows/WF08 并配 HeaderAuth

### 🚧 未做（下一冲刺，按规范排序）
- 社媒官方 API 适配（TikTok/Meta/YouTube 审批后自动发布——HUMAN_QUEUE 已是合规兜底）
- 广告中心（GA4/GSC 数据接入）、选品中心、内容编辑器与发布日历
- 内容编辑器、发布日历、多租户 SaaS 化（tenant_id 已预留）

## 红线（代码级强制）

1. **AI 永不自主对外发布**——APPROVAL 级工具未获批准直接拒绝（APPROVAL_REQUIRED→自动转审批）
2. 内容发布必须先 APPROVED 状态，否则 API 拒绝
3. 企业记忆不可由 Agent 直接写生效（PENDING 门）
4. 线索按邮箱去重，不重复建客
5. 失败可重试、幂等键防重复副作用、全程审计

## 服务器与端口

| 服务 | 端口 | 说明 |
|---|---|---|
| backend | 8600 | Rust API |
| frontend | 8610 | Next.js |
| n8n | 3210 | 调度执行层 |
| postgres | 5434 | pgvector/pgvector:pg16（容器 ai-workforce-pg，避开 deyou-pg:5433） |
