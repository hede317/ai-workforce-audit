好。这次我直接把它作为一个正式的软件项目来建设，不再拆成零散建议。

项目名称确定为 HedeYou Marketing OS。

我会按照可以交给 GPT、Claude Code、Codex 或专业开发团队执行的标准来编写。不是简单的产品介绍，而是一套完整的开发蓝图，包含产品需求、系统架构、数据库、接口、自动化工作流、管理后台、部署、测试和验收标准。

有一个重要原则：不推倒你已经做好的 hedeyou.com、shop、learn、study、ERP+CRM 和 AI 平台，而是在它们上面建立统一营销自动化中枢。

第一期重点是内容工厂，第二期接流量和客户，第三期扩展广告、邮件、选品、客服及报表。这样既能先投入使用，也能逐步产品化。

# HedeYou Marketing OS

## AI-Powered Global Marketing Automation Platform

### 完整产品需求与技术开发实施规范 V1.0

**项目类型：** 企业级 AI 全域营销自动化平台\
**建设模式：** 自用验证 → 模块化产品 → SaaS商业化\
**第一阶段：** AI内容工厂 + TikTok + SEO/GEO\
**核心架构：** n8n + PostgreSQL + AI Models + HeyGen + FFmpeg + API Gateway\
**项目状态：** 待开发

---

# 第一部分：项目总体定义

## 1.1 项目目标

开发一套完整、可运行、可访问、可持续扩展的 AI 全域营销自动化系统。

系统服务于 HedeYou 现有业务：

1. 国际职业培训及教育。
2. 跨境电商。
3. 国际留学服务。
4. 企业数字化及 ERP 服务。
5. AI 企业服务。

系统的核心任务：

- 自动发现市场需求。
- 自动收集和管理内容选题。
- AI 自动生产文章、图片、视频和数字人内容。
- 自动生成多语言、多平台内容。
- SEO 和 GEO 内容优化。
- 对接 TikTok、Facebook、Instagram、YouTube。
- 管理多个品牌和账号。
- 自动收集网站访问及客户线索。
- 自动进入 CRM。
- 统计营销活动、流量和转化效果。

## 1.2 产品定位

不是普通的内容管理系统。

不是简单的视频生成工具。

不是单纯的 n8n 工作流集合。

而是一个统一管理内容生产、数字人 IP、搜索引擎营销、社交媒体、客户获取及数据分析的营销操作平台。

核心商业逻辑：

**AI 生产内容 → 多平台分发 → 搜索流量获取 → 客户线索收集 → CRM 转化 → 数据分析 → 自动优化。**

## 1.3 建设原则

1. 优先使用成熟开源技术。
2. 业务模块之间通过 API 通信。
3. n8n 负责流程编排，不承担核心业务数据库职责。
4. PostgreSQL 负责统一业务数据。
5. AI 模型支持多供应商切换。
6. 视频生产与主业务服务器分离。
7. 所有发布行为可以追踪。
8. 所有重要操作支持人工审批。
9. 所有工作流支持异常处理和重试。
10. 从第一天考虑多租户和 SaaS 扩展，但第一期不开发复杂计费系统。

---

# 第二部分：业务架构

## 2.1 业务组织结构

系统必须支持多个业务单元。

| Business ID | 业务名称    | 业务网站              |
| ----------- | ------- | ----------------- |
| EDU         | 国际教育培训  | learn.hedeyou.com |
| ECOM        | 跨境电商    | shop.hedeyou.com  |
| STUDY       | 留学服务    | study.hedeyou.com |
| ERP         | 企业数字化服务 | erp.hedeyou.com   |
| AI          | AI 企业服务 | ai.hedeyou.com    |

主站 hedeyou.com 作为品牌总入口。

上述业务单元必须可以独立设置：

- 品牌名称。
- 品牌 Logo。
- 品牌颜色。
- 目标市场。
- 目标客户。
- 内容语言。
- IP 人物。
- 社交媒体账号。
- 内容生产模板。
- SEO 关键词。
- CTA 转化入口。
- CRM 业务分类。

## 2.2 目标市场

第一期优先支持：

- 中国。
- 巴基斯坦。
- 孟加拉国。
- 新加坡。
- 其他英语市场。

语言体系：

- 中文简体。
- 英语。
- 后续可扩展乌尔都语、孟加拉语、阿拉伯语等。

不同语言必须保留独立内容版本，不能简单依赖机器翻译覆盖原文。

---

# 第三部分：系统技术架构

## 3.1 总体架构

采用分层、模块化架构。

### A. 前端应用层

建议：

- Next.js。
- React。
- TypeScript。
- Tailwind CSS。
- shadcn/ui。

负责：

- 管理后台。
- 内容生产管理。
- IP 管理。
- 视频管理。
- 发布审批。
- SEO 管理。
- 数据报表。
- 系统配置。

### B. 后端业务层

建议：

- Python FastAPI。
- PostgreSQL。
- SQLAlchemy。
- Alembic。
- Redis（任务量增加后启用）。
- S3 兼容对象存储。

FastAPI 负责：

- 用户认证。
- 业务管理。
- 内容管理。
- 任务管理。
- 文件管理。
- 社交平台账号管理。
- 发布任务。
- 客户线索。
- 数据统计。
- 对外 API。

### C. 自动化层

采用 n8n。

负责：

- 定时任务。
- AI 内容生产。
- 内容审核触发。
- 视频生成任务。
- 网站发布。
- 社交媒体发布。
- SEO 检查。
- CRM 数据同步。
- 数据报表。
- 异常告警。

n8n 不直接取代后端业务服务。

### D. AI 服务层

采用统一 AI Provider Adapter。

支持：

- OpenAI。
- Anthropic Claude。
- 其他兼容 API 的模型供应商。
- 后续本地部署模型。

统一实现：

- 文本生成。
- 内容分类。
- 内容总结。
- 多语言翻译。
- 内容质量评分。
- 关键词分析。
- 视频脚本生成。
- 客户意向分析。

### E. 视频生产层

建议：

- HeyGen API。
- FFmpeg。
- Python。
- 对象存储。

负责：

- 数字人生成。
- 口播视频。
- 字幕生成。
- 视频画幅转换。
- 音频处理。
- 封面生成。
- 多平台视频版本。
- 视频文件归档。

视频渲染不得长期占用主业务服务器资源。

### F. 数据分析层

接入：

- Google Analytics 4。
- Google Search Console。
- Google Tag Manager。
- TikTok 平台数据接口。
- Meta 平台数据接口。
- YouTube Data API。

统一进行营销数据统计。

## 3.2 系统结构图

```
flowchart TD
    A[HedeYou Marketing OS] --> B[管理后台]
    A --> C[业务API]
    A --> D[n8n自动化中枢]

    B --> C
    D --> C

    C --> E[PostgreSQL]
    C --> F[对象存储]
    C --> G[Redis任务队列]

    D --> H[AI模型服务]
    D --> I[HeyGen数字人]
    D --> J[FFmpeg视频处理]

    D --> K[SEO/GEO]
    D --> L[社交媒体发布]
    D --> M[网站内容发布]

    M --> N[HedeYou网站矩阵]
    L --> O[TikTok]
    L --> P[Meta]
    L --> Q[YouTube]

    N --> R[GA4及GSC]
    O --> S[营销数据中心]
    P --> S
    Q --> S
    R --> S

    S --> T[CRM及经营报表]
```

---

# 第四部分：功能模块详细需求

## 4.1 模块一：统一经营仪表盘

系统登录后的首页。

展示：

- 今日生产文章数量。
- 今日生产视频数量。
- 待审核内容。
- 已发布内容。
- 发布失败数量。
- 各业务内容产量。
- 各平台账号表现。
- 网站访问量。
- 搜索点击量。
- 客户咨询量。
- 新增线索。
- 营销费用。
- 转化情况。

支持：

- 今日。
- 昨日。
- 最近7天。
- 最近30天。
- 自定义时间范围。

支持按业务、市场、语言、渠道筛选。

## 4.2 模块二：AI选题中心

功能：

1. 业务关键词库。
2. 用户问题库。
3. 搜索需求分析。
4. 竞品内容研究。
5. 社交媒体话题采集。
6. 历史内容分析。
7. AI选题生成。
8. 选题去重。
9. 选题价值评分。
10. 内容日历。

选题评分维度：

| 评分项目    | 权重建议 |
| ------- | ---- |
| 目标客户相关度 | 30%  |
| 搜索需求    | 25%  |
| 商业转化潜力  | 25%  |
| 内容差异化   | 10%  |
| 生产难度    | 10%  |

所有分数由 AI 生成初评，允许人工修改。

每个选题必须包含：

- Topic ID。
- 业务分类。
- 标题。
- 目标用户。
- 目标市场。
- 目标语言。
- 关键词。
- 搜索意图。
- 内容角度。
- CTA。
- 优先级。
- 状态。

状态：

NEW → ANALYZED → APPROVED → PRODUCING → COMPLETED。

## 4.3 模块三：AI文章生产中心

支持生产：

- SEO 博客文章。
- GEO 问答内容。
- 企业案例。
- 产品介绍。
- 培训课程介绍。
- 行业分析。
- FAQ。
- 营销落地页。

文章生产流程：

1. 读取选题。
2. 读取对应业务知识库。
3. 检索可信来源。
4. 生成大纲。
5. 生成文章。
6. 检查事实和来源。
7. 检查重复内容。
8. 生成标题。
9. 生成 Meta Description。
10. 生成 FAQ。
11. 生成内部链接建议。
12. 生成结构化数据建议。
13. 人工审核。
14. 发布到指定网站。

文章编辑器支持：

- Markdown。
- 富文本。
- SEO 字段。
- 多语言版本。
- 版本记录。
- 发布状态。
- 内容关联。

## 4.4 模块四：数字人 IP 管理中心

第一期创建四个 IP。

| IP   | 业务方向    | 内容主题        |
| ---- | ------- | ----------- |
| IP01 | 企业数字化顾问 | ERP、AI、流程管理 |
| IP02 | 国际教育顾问  | 留学、学校、教育    |
| IP03 | 职场培训讲师  | 中文、技能、职场    |
| IP04 | 跨境商业观察  | 商品、市场、商业    |

每个 IP 必须配置：

- IP ID。
- 人物名称。
- 人物定位。
- 目标受众。
- 人物形象。
- 数字人供应商 ID。
- 声音 ID。
- 语言。
- 语气。
- 内容风格。
- 品牌规范。
- 视频模板。
- 关联业务。

支持：

- 创建 IP。
- 编辑 IP。
- 复制 IP 配置。
- 启用和停用。
- IP 内容统计。
- IP 视频资产管理。

数字人形象和声音必须具备合法授权。

## 4.5 模块五：视频自动化生产工厂

输入：

- 文章 ID。
- 视频脚本。
- IP ID。
- 目标语言。
- 视频模板。

自动完成：

1. 生成口播脚本。
2. 生成分镜。
3. 调用数字人 API。
4. 生成视频。
5. 获取视频文件。
6. 自动加字幕。
7. 自动调整视频画幅。
8. 自动生成封面。
9. 自动生成标题。
10. 生成平台描述。
11. 生成标签。
12. 保存素材。

视频规格模板：

| 平台              | 默认比例       |
| --------------- | ---------- |
| TikTok          | 9:16       |
| Instagram Reels | 9:16       |
| YouTube Shorts  | 9:16       |
| YouTube 长视频     | 16:9       |
| Facebook        | 按投放及内容规格配置 |

视频生产任务必须支持：

- 任务状态。
- 生成进度。
- 失败原因。
- 重试。
- 文件地址。
- 生成成本。
- 供应商任务 ID。

视频任务状态：

PENDING → PROCESSING → COMPLETED / FAILED。

## 4.6 模块六：多平台发布中心

第一期优先：

1. TikTok。
2. Facebook。
3. Instagram。
4. YouTube。
5. 自有网站。

支持：

- 多账号管理。
- 账号授权。
- 内容排期。
- 发布任务。
- 平台内容格式转换。
- 标题及描述。
- 标签。
- 发布审批。
- 发布状态。
- 发布链接。
- 失败重试。
- 数据回收。

重要要求：

所有社交平台发布必须通过官方授权接口或平台允许的人工发布流程。

不能通过绕过平台权限、模拟登录等方式实现未经授权的自动发布。

TikTok 发布接口必须考虑应用审核、账号授权及公开发布权限。

第一期必须具备人工上传备选机制。

发布状态：

DRAFT → APPROVED → SCHEDULED → PUBLISHING → PUBLISHED / FAILED。

## 4.7 模块七：SEO + GEO 中心

SEO 功能：

- 关键词库。
- 关键词分类。
- 搜索意图。
- 文章 SEO 检查。
- Title 检查。
- Description 检查。
- Heading 检查。
- 内部链接检查。
- 图片 Alt 检查。
- Canonical 检查。
- Sitemap 检查。
- Schema 检查。
- 索引监控。

GEO 功能：

- 目标问题库。
- 品牌实体资料。
- 业务实体资料。
- FAQ。
- 专业知识文章。
- 内容来源管理。
- 内容事实核查。
- AI 搜索可见性跟踪。

GEO 不能承诺特定 AI 平台一定引用网站内容。

Google Search Console 负责搜索表现及索引数据。

Google Analytics 4 负责用户行为及转化数据。

## 4.8 模块八：客户线索中心

统一接入：

- 主站。
- 培训网站。
- 电商网站。
- 留学网站。
- ERP网站。
- AI网站。
- 社交媒体咨询入口。
- 广告落地页。

客户字段：

- Lead ID。
- 姓名。
- 邮箱。
- 电话。
- 国家。
- 业务类型。
- 客户需求。
- 来源渠道。
- 来源内容 ID。
- 营销活动 ID。
- 创建时间。
- 跟进状态。
- 客户授权记录。

自动分类：

EDU / ECOM / STUDY / ERP / AI。

线索状态：

NEW → QUALIFIED → CONTACTED → NEGOTIATION → WON / LOST。

客户信息必须进行权限控制，不能因为进入统一数据库就默认可以向所有营销渠道发送邮件。

## 4.9 模块九：数据分析中心

统计：

- 内容生产成本。
- 单条视频成本。
- 各平台播放量。
- 文章阅读量。
- 网站访问量。
- 搜索点击量。
- 点击率。
- 咨询量。
- 有效线索量。
- 成交量。
- 广告成本。
- 获客成本。
- 转化率。

每日自动生成经营日报。

日报内容：

1. 今日内容生产情况。
2. 今日发布情况。
3. 各平台数据。
4. 网站流量。
5. 搜索表现。
6. 新增客户。
7. 工作流异常。
8. 明日建议。

---

# 第五部分：数据库设计

采用 PostgreSQL。

所有主要业务表应具备：

- id。
- tenant_id。
- created_at。
- updated_at。
- created_by。
- status（适用时）。

第一期采用单租户运行，但数据库设计预留 tenant_id。

## 5.1 核心表

### organizations

| 字段     | 类型      |
| ------ | ------- |
| id     | UUID    |
| name   | VARCHAR |
| domain | VARCHAR |
| status | VARCHAR |

### business_units

| 字段               | 类型      |
| ---------------- | ------- |
| id               | UUID    |
| code             | VARCHAR |
| name             | VARCHAR |
| website          | VARCHAR |
| description      | TEXT    |
| default_language | VARCHAR |
| status           | VARCHAR |

### social_accounts

| 字段                | 类型        |
| ----------------- | --------- |
| id                | UUID      |
| platform          | VARCHAR   |
| account_name      | VARCHAR   |
| platform_user_id  | VARCHAR   |
| access_token_ref  | VARCHAR   |
| refresh_token_ref | VARCHAR   |
| token_expires_at  | TIMESTAMP |
| status            | VARCHAR   |

令牌必须使用加密凭据存储，不能明文保存。

### ip_profiles

| 字段               | 类型      |
| ---------------- | ------- |
| id               | UUID    |
| business_unit_id | UUID    |
| name             | VARCHAR |
| positioning      | TEXT    |
| avatar_provider  | VARCHAR |
| avatar_id        | VARCHAR |
| voice_id         | VARCHAR |
| language         | VARCHAR |
| configuration    | JSONB   |
| status           | VARCHAR |

### content_topics

| 字段               | 类型      |
| ---------------- | ------- |
| id               | UUID    |
| business_unit_id | UUID    |
| title            | VARCHAR |
| target_audience  | TEXT    |
| target_market    | VARCHAR |
| language         | VARCHAR |
| keywords         | JSONB   |
| search_intent    | VARCHAR |
| priority         | INTEGER |
| status           | VARCHAR |

### content_assets

| 字段               | 类型      |
| ---------------- | ------- |
| id               | UUID    |
| topic_id         | UUID    |
| business_unit_id | UUID    |
| content_type     | VARCHAR |
| title            | VARCHAR |
| body             | TEXT    |
| language         | VARCHAR |
| source_refs      | JSONB   |
| seo_metadata     | JSONB   |
| status           | VARCHAR |

### content_versions

| 字段               | 类型        |
| ---------------- | --------- |
| id               | UUID      |
| content_asset_id | UUID      |
| version_number   | INTEGER   |
| content          | JSONB     |
| change_summary   | TEXT      |
| created_at       | TIMESTAMP |

### media_assets

| 字段               | 类型      |
| ---------------- | ------- |
| id               | UUID    |
| content_asset_id | UUID    |
| media_type       | VARCHAR |
| storage_key      | VARCHAR |
| mime_type        | VARCHAR |
| file_size        | BIGINT  |
| duration         | FLOAT   |
| metadata         | JSONB   |

### publishing_tasks

| 字段                | 类型        |
| ----------------- | --------- |
| id                | UUID      |
| content_asset_id  | UUID      |
| social_account_id | UUID      |
| platform          | VARCHAR   |
| scheduled_at      | TIMESTAMP |
| status            | VARCHAR   |
| idempotency_key   | VARCHAR   |
| error_message     | TEXT      |

### publishing_results

| 字段                 | 类型        |
| ------------------ | --------- |
| id                 | UUID      |
| publishing_task_id | UUID      |
| platform_post_id   | VARCHAR   |
| published_url      | TEXT      |
| published_at       | TIMESTAMP |
| metrics            | JSONB     |

### leads

| 字段                | 类型      |
| ----------------- | ------- |
| id                | UUID    |
| business_unit_id  | UUID    |
| name              | VARCHAR |
| email             | VARCHAR |
| phone             | VARCHAR |
| country           | VARCHAR |
| source_channel    | VARCHAR |
| source_content_id | UUID    |
| consent_status    | VARCHAR |
| lead_status       | VARCHAR |
| metadata          | JSONB   |

### marketing_campaigns

| 字段               | 类型      |
| ---------------- | ------- |
| id               | UUID    |
| business_unit_id | UUID    |
| name             | VARCHAR |
| channel          | VARCHAR |
| start_date       | DATE    |
| end_date         | DATE    |
| budget           | NUMERIC |
| status           | VARCHAR |

### workflow_executions

| 字段                    | 类型        |
| --------------------- | --------- |
| id                    | UUID      |
| workflow_code         | VARCHAR   |
| external_execution_id | VARCHAR   |
| business_entity_id    | UUID      |
| status                | VARCHAR   |
| started_at            | TIMESTAMP |
| finished_at           | TIMESTAMP |
| error_message         | TEXT      |

### audit_logs

记录：

- 用户操作。
- 审批记录。
- 发布操作。
- 配置修改。
- 权限修改。
- API调用关键事件。

注意：客户个人信息、访问令牌和敏感配置不能直接写入普通运行日志。

## 5.2 数据关系

```
erDiagram
    BUSINESS_UNITS ||--o{ CONTENT_TOPICS : owns
    CONTENT_TOPICS ||--o{ CONTENT_ASSETS : generates
    CONTENT_ASSETS ||--o{ CONTENT_VERSIONS : has
    CONTENT_ASSETS ||--o{ MEDIA_ASSETS : has
    CONTENT_ASSETS ||--o{ PUBLISHING_TASKS : publishes
    SOCIAL_ACCOUNTS ||--o{ PUBLISHING_TASKS : receives
    PUBLISHING_TASKS ||--o{ PUBLISHING_RESULTS : records
    BUSINESS_UNITS ||--o{ LEADS : receives
    CONTENT_ASSETS ||--o{ LEADS : generates
    BUSINESS_UNITS ||--o{ MARKETING_CAMPAIGNS : owns
```

---

# 第六部分：API 接口规范

统一 API 前缀：

`/api/v1`

统一采用 JSON。

统一返回格式：

```
{
  "success": true,
  "data": {},
  "error": null,
  "request_id": "uuid"
}
```

## 6.1 业务单元接口

```
GET    /business-units
POST   /business-units
GET    /business-units/{id}
PUT    /business-units/{id}
```

## 6.2 选题接口

```
GET    /topics
POST   /topics
GET    /topics/{id}
PUT    /topics/{id}
POST   /topics/{id}/approve
POST   /topics/{id}/generate
```

## 6.3 内容接口

```
GET    /contents
POST   /contents
GET    /contents/{id}
PUT    /contents/{id}
POST   /contents/{id}/review
POST   /contents/{id}/approve
POST   /contents/{id}/publish
```

## 6.4 视频接口

```
POST   /videos/generate
GET    /videos/{id}
POST   /videos/{id}/retry
GET    /videos/{id}/download
```

视频生成必须采用异步任务机制。

## 6.5 发布接口

```
GET    /publishing/tasks
POST   /publishing/tasks
GET    /publishing/tasks/{id}
POST   /publishing/tasks/{id}/approve
POST   /publishing/tasks/{id}/cancel
POST   /publishing/tasks/{id}/retry
```

## 6.6 客户接口

```
GET    /leads
POST   /leads
GET    /leads/{id}
PUT    /leads/{id}
POST   /leads/{id}/assign
POST   /leads/{id}/events
```

## 6.7 工作流接口

```
POST   /workflows/trigger
GET    /workflows/executions
GET    /workflows/executions/{id}
POST   /workflows/executions/{id}/retry
```

所有写入接口需要认证和权限检查。

需要幂等控制的接口使用：

`Idempotency-Key`

---

# 第七部分：n8n 工作流设计

第一期必须建立以下八条工作流。

## WF01：每日自动选题

Trigger：

每天定时运行。

Nodes：

1. Schedule Trigger。
2. HTTP Request：读取业务配置。
3. PostgreSQL：获取关键词。
4. HTTP Request：采集允许访问的数据源。
5. AI Model：选题分析。
6. Code：标准化数据。
7. PostgreSQL：重复检测。
8. PostgreSQL：写入选题库。
9. Notification：通知审核。

输出：

每日10个候选选题。

## WF02：AI文章生产

Trigger：

选题审批通过。

Nodes：

1. Webhook。
2. PostgreSQL：读取选题。
3. Knowledge Retrieval。
4. AI：生成文章大纲。
5. AI：生成文章。
6. AI：生成FAQ。
7. AI：生成SEO字段。
8. AI：检查事实及来源。
9. PostgreSQL：保存内容。
10. Notification：进入人工审核。

输出：

完整文章及SEO数据。

## WF03：数字人视频生成

Trigger：

文章审批通过。

Nodes：

1. Webhook。
2. PostgreSQL：读取文章。
3. AI：生成视频脚本。
4. AI：生成分镜。
5. HTTP Request：调用HeyGen。
6. Wait/Poll：查询视频任务状态。
7. HTTP Request：获取生成视频。
8. Execute Command 或独立渲染服务：FFmpeg。
9. S3：上传素材。
10. PostgreSQL：记录文件信息。
11. Notification：视频审核。

输出：

可供发布的视频文件。

注意：视频任务应采用异步轮询或回调，不应长时间占用 n8n 执行线程。

## WF04：多平台内容发布

Trigger：

发布审批通过。

Nodes：

1. Webhook。
2. PostgreSQL：读取发布任务。
3. IF：检查账号权限。
4. IF：检查平台格式。
5. HTTP Request：调用平台官方API。
6. PostgreSQL：保存平台ID。
7. PostgreSQL：保存发布地址。
8. Error Trigger：处理异常。

API不支持自动发布的账号，进入人工发布队列。

## WF05：SEO/GEO检查

Trigger：

文章发布成功。

检查：

- 页面是否可访问。
- Title 是否存在。
- Description 是否存在。
- H1 是否合理。
- 图片 Alt。
- 内部链接。
- Canonical。
- Schema。
- Sitemap。
- FAQ。
- 内容来源。

结果保存数据库。

## WF06：网站客户线索采集

Trigger：

网站表单Webhook。

Nodes：

1. Webhook。
2. 数据格式校验。
3. 垃圾信息过滤。
4. 联系方式去重。
5. 识别业务类型。
6. 写入 leads。
7. 记录来源。
8. 通知业务负责人。

## WF07：客户自动分类

Trigger：

新客户进入CRM。

分析：

- 客户所属业务。
- 目标国家。
- 需求类别。
- 购买意向。
- 联系优先级。

AI只提供辅助分类，重要客户分配和对外承诺由人工确认。

## WF08：每日经营报表

Trigger：

每天固定时间。

采集：

- 内容生产。
- 视频生产。
- 发布状态。
- 网站访问。
- 搜索数据。
- 新客户。
- 转化情况。
- 工作流异常。

生成 Markdown 报告，并通过邮件或现有通知渠道发送。

---

# 第八部分：SEO、GEO 与流量追踪规范

## 8.1 SEO

接入：

- Google Search Console API。
- Google Analytics Data API。
- Google Tag Manager。
- 网站 Sitemap。

建立关键词分组：

- 品牌关键词。
- 产品关键词。
- 服务关键词。
- 问题型关键词。
- 地域关键词。
- 商业意图关键词。

每篇文章必须保留关键词和搜索意图记录。

## 8.2 GEO

建立品牌知识库：

- 公司介绍。
- 业务介绍。
- 产品资料。
- 服务范围。
- 价格政策。
- 真实案例。
- 常见问题。
- 联系方式。
- 权威来源。

所有信息必须可以追溯到具体来源和更新时间。

## 8.3 UTM

所有营销链接采用统一规则：

```
utm_source=tiktok
utm_medium=social
utm_campaign=erp_ai_2026
utm_content=ip01_video001
```

统一记录：

source、medium、campaign、content、term。

## 8.4 转化事件

建议建立：

```
page_view
view_content
lead_form_start
lead_form_submit
consultation_request
course_registration
purchase
```

不同网站根据业务实际情况设置事件。

不得将普通页面浏览误判为成交。

---

# 第九部分：管理后台页面结构

前端必须实现以下页面。

| 页面               | 功能     |
| ---------------- | ------ |
| /dashboard       | 经营仪表盘  |
| /business-units  | 业务管理   |
| /topics          | 选题管理   |
| /calendar        | 内容日历   |
| /contents        | 内容管理   |
| /contents/editor | 内容编辑器  |
| /ip-profiles     | 数字人IP  |
| /videos          | 视频生产中心 |
| /publishing      | 发布管理   |
| /accounts        | 社交账号   |
| /seo             | SEO管理  |
| /geo             | GEO知识库 |
| /leads           | 客户线索   |
| /campaigns       | 营销活动   |
| /analytics       | 数据分析   |
| /workflows       | 工作流管理  |
| /settings        | 系统配置   |

管理后台必须具备：

- 登录认证。
- 角色权限。
- 表格筛选。
- 搜索。
- 分页。
- 状态显示。
- 操作确认。
- 异常提示。
- 审批记录。

第一期至少建立三种角色：

ADMIN：系统管理员。

EDITOR：内容编辑及审批。

OPERATOR：日常运营。

---

# 第十部分：安全及稳定性要求

1. 所有管理接口必须认证。
2. 使用 HTTPS。
3. API密钥使用加密存储。
4. 社交账号授权令牌加密保存。
5. 数据库不能直接暴露公网。
6. n8n管理界面必须有访问保护。
7. 设置自动备份。
8. 工作流执行必须有日志。
9. 失败任务支持有限次数重试。
10. 发布任务必须有幂等机制。
11. 记录重要人工审批操作。
12. 设置API调用频率限制。
13. 防止Webhook重复提交。
14. 保护客户隐私和营销授权记录。
15. 遵循目标国家适用的数据保护及营销邮件规定。

建议初始备份策略：

- 每日数据库备份。
- 每周完整备份。
- 备份异地保存。
- 定期进行恢复演练。

---

# 第十一部分：部署架构

## 11.1 初期部署

建议服务：

```
marketing.hedeyou.com
api-marketing.hedeyou.com
n8n.hedeyou.com
```

服务组成：

```
Docker Compose
│
├── frontend
├── backend
├── postgres
├── n8n
├── reverse-proxy
└── monitoring
```

Redis在任务规模需要时启用。

视频渲染服务单独部署。

对象存储优先采用S3兼容服务。

## 11.2 服务器要求

开发环境：

- 2核CPU。
- 4GB RAM。
- 40GB以上存储。

仅适合轻量原型、少量工作流测试。

正式环境建议：

- 4核CPU起步。
- 8GB RAM起步。
- SSD存储。
- 数据库独立备份。
- 视频渲染独立节点。

最终配置根据实际并发和视频生成量调整。

## 11.3 监控

接入：

- Prometheus。
- Grafana。
- Zabbix（已有环境可复用）。

重点监控：

- CPU。
- RAM。
- 磁盘。
- 数据库连接。
- API响应时间。
- n8n执行失败。
- 视频生成失败。
- 社交平台API异常。

---

# 第十二部分：开发阶段及时间安排

## Sprint 1：系统基础

目标：建立可运行的系统骨架。

任务：

- 项目仓库。
- Docker Compose。
- 前端框架。
- FastAPI。
- PostgreSQL。
- 数据库迁移。
- 登录权限。
- n8n连接。
- AI模型配置。
- 日志和异常处理。

交付：

可登录、可管理业务单元、可调用AI的系统。

## Sprint 2：内容工厂

目标：跑通内容生产。

任务：

- 选题管理。
- AI文章生成。
- 多语言内容。
- IP配置。
- 视频脚本。
- HeyGen接口。
- FFmpeg。
- 素材存储。
- 审核中心。

交付：

从选题到文章、视频的完整生产流程。

## Sprint 3：发布与搜索优化

目标：内容真正上线。

任务：

- 网站发布API。
- TikTok账号授权。
- TikTok发布能力。
- Meta接口。
- YouTube接口。
- 发布队列。
- SEO检查。
- Search Console。
- 内容数据回收。

交付：

从生产到审批再到发布的完整闭环。

## Sprint 4：数据和客户转化

目标：建立营销反馈系统。

任务：

- GA4。
- GTM。
- UTM。
- 网站表单接入。
- CRM同步。
- 客户分类。
- 数据报表。
- 每日经营日报。

交付：

从内容到客户的完整数据链路。

---

# 第十三部分：第一期验收标准

以下为必须通过的验收项目。

| 编号  | 验收项目  | 通过标准                  |
| --- | ----- | --------------------- |
| T01 | 系统登录  | 可以正常登录                |
| T02 | 业务管理  | 四个业务可独立管理             |
| T03 | 选题生成  | 自动生成候选选题              |
| T04 | 文章生成  | 成功生成完整文章              |
| T05 | 多语言   | 中文和英文版本可生成            |
| T06 | IP管理  | 可以配置不同数字人             |
| T07 | 视频生成  | 可以生成并保存视频             |
| T08 | 字幕处理  | 可以自动生成字幕成片            |
| T09 | 内容审核  | 可以审核和退回               |
| T10 | 网站发布  | 可以发布到指定网站             |
| T11 | 社交发布  | 至少一个账号完成合法授权发布或人工发布流程 |
| T12 | SEO检查 | 可以生成SEO检查结果           |
| T13 | 客户采集  | 可以将表单写入线索数据库          |
| T14 | 数据统计  | 可以查看基本营销数据            |
| T15 | 异常处理  | 可以查看失败任务及重试           |
| T16 | 数据安全  | 令牌不明文存储，数据库可备份恢复      |

## 13.1 运营验收目标

试运行阶段：

- 每日10个选题。
- 每日1～2篇文章。
- 每日3～5条视频。
- 中文、英文两种语言。
- 四个业务独立分类。
- 每篇文章有SEO检查结果。
- 所有内容经过人工审批。
- 每条内容有发布记录。
- 每条客户线索可追溯来源。
- 每日自动生成经营日报。

---

# 第十四部分：第二阶段扩展规划

第一期稳定运行后，增加以下模块。

## 14.1 邮件自动营销

功能：

- 企业客户发现。
- 目标企业筛选。
- 联系人信息整理。
- 企业客户画像。
- 邮件内容生成。
- 邮件发送。
- 自动跟进。
- 回复识别。
- CRM同步。

必须使用合法来源的商业联系人信息，遵循适用的营销邮件法规，提供退订机制。

不得通过无差别批量发送垃圾邮件的方式获客。

## 14.2 Google Ads + Meta Ads

功能：

- 广告账户管理。
- 广告系列管理。
- 预算记录。
- 广告效果采集。
- CPC。
- CPM。
- CTR。
- CPL。
- 转化率。
- ROI。

第一阶段只读数据及生成优化建议。

预算修改、广告启停和自动扩量必须经过人工授权。

## 14.3 自动化选品

功能：

- 商品数据采集。
- 竞争产品分析。
- 市场需求分析。
- 价格分析。
- 毛利率估算。
- 物流成本。
- 供应商管理。
- 产品机会评分。

## 14.4 AI客服和客诉

功能：

- FAQ知识库。
- 多语言客服。
- 客户问题分类。
- 订单状态查询。
- 客诉工单。
- 自动分配。
- 服务质量分析。

涉及退款、赔偿、账户变更等高风险动作必须人工审批。

## 14.5 SaaS产品化

未来支持：

- 多租户。
- 用户注册。
- 订阅套餐。
- API配额。
- 工作流额度。
- AI Token成本核算。
- 独立品牌。
- 独立域名。
- 租户数据隔离。
- 账单管理。

第一期只预留必要的数据结构，不开发复杂计费体系。

---

# 第十五部分：开发团队执行规范

本项目采用 AI Agent 辅助开发。

建议使用：

- Claude Code。
- Codex。
- GPT。
- GitHub。

开发要求：

1. 先创建项目目录。
2. 先建立数据库模型。
3. 先完成基础API。
4. 再完成前端管理后台。
5. 再接入n8n。
6. 再接入AI模型。
7. 再接入视频生产。
8. 再完成社交平台接口。
9. 最后完成端到端测试。

每个模块必须：

- 有独立代码目录。
- 有接口文档。
- 有数据库迁移。
- 有错误处理。
- 有基本测试。
- 有运行说明。

所有开发任务通过 Git 分支和 Pull Request 管理。

禁止：

- 只生成静态页面。
- 用假数据冒充真实接口。
- 用按钮模拟尚未实现的功能。
- 将API密钥硬编码。
- 将所有业务逻辑放入一个n8n工作流。
- 在没有授权时自动发布。
- 省略错误处理和日志。

---

# 第十六部分：最终交付清单

开发完成后，必须交付：

1. 完整前端源代码。
2. 完整后端源代码。
3. PostgreSQL数据库设计。
4. Alembic数据库迁移文件。
5. Docker Compose部署文件。
6. 环境变量示例文件。
7. n8n工作流JSON导出文件。
8. AI Prompt模板。
9. HeyGen接口集成。
10. FFmpeg视频处理模块。
11. 社交平台API适配模块。
12. 网站发布接口。
13. SEO检查模块。
14. CRM数据同步模块。
15. 自动化报表模块。
16. API接口文档。
17. 部署说明。
18. 系统操作手册。
19. 测试报告。
20. 数据备份及恢复说明。

**最终交付必须是真实可运行的软件，而不是静态原型、设计图或PRD演示页面。**

---

# 第十七部分：给开发 Agent 的最终执行指令

你现在是一名资深软件架构师、全栈工程师和AI自动化工程师。

请根据以上完整技术规范，直接开发 HedeYou Marketing OS。

你的第一目标不是开发一个复杂的商业SaaS，而是先建立一个可以真正投入使用的内部营销自动化系统。

请按照以下优先顺序执行：

**P0：系统基础**

- Docker部署。
- 前后端。
- 数据库。
- 登录权限。
- n8n连接。

**P1：内容工厂**

- 选题。
- 文章。
- AI内容生成。
- IP管理。
- 视频生产。
- 人工审核。

**P2：内容分发**

- 网站发布。
- TikTok。
- Meta。
- YouTube。
- SEO/GEO。

**P3：营销闭环**

- GA4。
- Search Console。
- UTM。
- CRM。
- 客户线索。
- 经营报表。

开发过程中遵循：

1. 每个阶段完成后进行实际运行测试。
2. 不要只输出代码片段，必须创建完整项目。
3. 不要使用虚假数据代替生产接口。
4. 外部服务未授权时提供真实的待配置状态。
5. 任何未完成模块必须明确标注。
6. 优先保证核心流程完整运行。
7. 不得因为追求界面效果而牺牲实际业务功能。
8. 代码必须可以部署、维护和二次开发。

**第一期最重要的验收场景：**

用户登录 HedeYou Marketing OS，选择企业数字化业务，输入一个选题，系统自动生成文章、视频脚本、数字人视频、字幕及平台发布素材，经过人工审核后发布到网站和授权的社交媒体账号，最后记录内容表现和客户咨询来源。

只要这个完整场景跑通，第一期就具备了真正的业务价值。

请先创建项目结构、数据库和基础服务，然后逐步实现，不要一次性生成大量无法运行的代码。

**项目最终目标：建立一个由 AI 驱动、能够持续生产内容、持续获取客户、持续优化营销效果的全球业务增长系统。**

## 接下来怎么落地

我建议实际开发时不要一次性把整份文档交给 Agent，让它试图一次生成所有代码。应当按 Sprint 1 → Sprint 2 → Sprint 3 → Sprint 4 顺序执行，每个阶段运行验收后再进入下一阶段。

另外，有三件事情需要在部署前核实：

1. 现有各网站是否都有可用的内容发布 API。
2. HeyGen 的 API 账号及数字人资源是否可用。
3. TikTok、Meta、Google 的开发者应用和账号授权情况。

你现有的自动化、ERP+CRM、AI平台及监控基础可以复用，具体接口需要开发时逐一验证。

我的建议：先把第一期做成一个真正能运行的内容工厂。 不急着购买更多服务器或广告预算。先让它连续运行7天，确认每天能稳定生成、审核和发布内容，再扩大规模。

下一步就可以把这份规范交给开发 Agent，开始创建真实项目代码。