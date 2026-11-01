# PR#1 范围修订：HedeYou AI Marketing Agency OS（修改计划 v1）

> 依据：审计方《PR#1 业务范围修订指令》+ 2026-11-01 战略北极星。
> 本文档 = 修改计划 + 数据库变化 + API 变化。**未经审查不写迁移/不改代码**（指令第7条）。

## 一、范围声明（修订后）

**只建设**：AI 驱动的营销获客与代运营工作平台——一个人/小团队通过 AI Agent 完成多家企业客户的营销代运营（客户开发签约→研究→策略→内容→视频→SEO/GEO→社媒→广告→线索→CRM→分析→交付报告→续约）。

**明确排除**（不开发/不集成/不管理）：矿山 MES/SIMS/无人值守、纺织 MES、智慧水产、ESP32/工业 IoT 等行业专用生产管理系统——属独立项目。平台可为这些行业的企业提供**通用营销功能**的代运营服务，但不建行业专用模块。

**允许保留**：客户自建通用产品/服务资料（名称/介绍/目标客户/价格/品牌/素材/卖点）——只用于营销生产，不构成行业系统。

## 二、核心业务对象变更：客户 Workspace

每个营销客户一个独立 Workspace（HedeYou 自有营销 = 同一能力的自有 Workspace）：

1. 客户资料 2. 品牌与语气 3. 产品/服务资料 4. 目标市场与画像 5. 营销策略 6. 活动 7. 社媒账号 8. 广告账户 9. 内容与审批 10. 线索 11. 效果数据 12. 月度交付报告

**隔离铁律**：数据/记忆/账号凭据/内容/线索按 workspace 隔离（权限+测试）。

## 三、数据库变化（实施时以迁移文件交付，草案）

```sql
-- 新增
clients(id, name, status, contact JSONB, created_at)
workspaces(id, client_id NULL, name, brand_profile JSONB, created_at)  -- client_id NULL = HedeYou 自有
marketing_projects(id, workspace_id, name, objective, brief JSONB, status)
social_accounts(id, workspace_id, platform, account_name, credentials_ref, status)
monthly_reports(id, workspace_id, period, markdown, metrics JSONB)
-- 隔离列（存量数据迁到 HedeYou 自有 workspace）
ALTER TABLE content_topics      ADD COLUMN workspace_id TEXT;
ALTER TABLE content_assets      ADD COLUMN workspace_id TEXT;
ALTER TABLE publishing_tasks    ADD COLUMN workspace_id TEXT;
ALTER TABLE leads               ADD COLUMN workspace_id TEXT;
ALTER TABLE campaigns           ADD COLUMN workspace_id TEXT;
ALTER TABLE listening_keywords  ADD COLUMN workspace_id TEXT;
ALTER TABLE listening_signals   ADD COLUMN workspace_id TEXT;
ALTER TABLE memories            ADD COLUMN workspace_id TEXT;  -- 记忆按客户隔离
```

## 四、API 变化（草案）

```
POST/GET/PUT /api/v1/workspaces          # 客户与自有工作区 CRUD
GET  /api/v1/workspaces/{id}/report      # 月度交付报告（P3 自动化）
全部业务列表接口增 ?workspace_id= 过滤；写入接口必填 workspace_id
现有 business_units（EDU/ECOM/STUDY/ERP）保留为内部品牌线数据，不再作为"客户"替代物
```

## 五、现有功能处置

| 现有 | 处置 |
|---|---|
| 舆情雷达/内容中心/审批/发布中心/线索/选品/CRM | 保留，挂 workspace_id（选品中心语义改为"客户产品与服务资料+机会评估"，仍通用） |
| botarmy 桥/listen 工具/MPT 出片 | 保留（通用营销能力） |
| GROWTH-OS-ALIGNMENT.md | v2 修订：solutions 行业目录撤销，改为通用"客户产品/服务资料"；行业产品集→独立项目引用 |
| 12+1 Agent | 保留；围绕代运营链路重排序（Marketing Director 化），不新增行业 Agent |

## 六、优先级（照指令）

P0 Workspace+品牌+项目+内容管理 → P1 研究/内容/SEO/审批/发布队列 → P2 CRM/归因/跟进/分析 → P3 官方 API+报告自动化 → P4 Agent 自主规划协作。

## 七、红线增量

所有对外发布/邮件/广告预算与投放保留人工审批（已有）；**新增**：跨 Workspace 数据访问=越权（权限+隔离测试为验收项）。
