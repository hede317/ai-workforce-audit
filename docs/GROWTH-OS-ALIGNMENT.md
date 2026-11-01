# ⚠️ 范围修订（审计方指令，本文件部分内容被取代）

> 本文件原按"四业务线 Growth OS"起草（含矿山/纺织/水产 solutions 行业目录）。
> 现按《PR#1 范围修订指令》收窄：**本仓库 = AI Marketing Agency OS，只做通用营销代运营**；
> 行业产品集属独立项目，不在本仓库建设。solutions 表设计由"客户产品/服务资料"（通用字段）替代。
> 保留价值：CRM 闭环/opportunities/marketing_briefs/驾驶舱指标设计仍然有效，实施时挂 workspace 隔离。

---
# Growth OS 业务对齐方案（三审第二阶段 · 设计稿，未经审查不写码）

> 依据：2026-11-01 战略北极星（获客先行/卖结果不卖软件/30天单一行业样板）。
> 本 PR 只交付设计，分阶段实施另行排期。

## 1. 商业定位
第一收入引擎 = AI Marketing & Growth 代运营（三档：基础内容运营/ai营销运营/企业获客——月费+基础费&绩效，不承诺订单数只承诺可控交付：内容数量/发布质量/客户研究/有效线索/沟通记录）。

## 2. 数据模型分层（区分三个概念）
- business_units：内部业务单元与品牌（现有 EDU/ECOM/STUDY/ERP 保留不动）
- solutions：对外销售的解决方案（新增表）——industry/痛点/方案描述/组成(软硬件)/交付内容/成熟状态/演示地址/报价模式/案例材料
- clients：客户工作区（新增表）——接受服务的企业客户，与 solutions 多对多（合同/服务档位）

## 3. 新表草案（实施时以迁移文件交付）
solutions(id, code, name, industry, pain_points JSONB, description, components JSONB, maturity, demo_url, pricing_model, case_materials JSONB, status)
clients(id, name, country, industry, contact JSONB, workspace_status)
client_contracts(id, client_id, solution_id, tier, monthly_fee, started_at, status)
opportunities(id, client_id, solution_id, stage[需求/演示/报价/谈判/WON/LOST], amount, lost_reason, next_follow_at, owner)
marketing_briefs(id, goal_text, country, industry, customer_type, product_ref, channel, language, objective, budget, approval_id)

## 4. 营销闭环链路
Marketing Brief → Market Research(舆情雷达复用) → Content Production(内容中心) → Human Approval(审批中心) → Publishing Queue(发布中心) → Lead Capture(线索中心) → CRM(leads+opportunities) → Proposal → WON/LOST → Revenue Analytics(日报+经营驾驶舱)

## 5. CRM 销售闭环增补
leads 增：source_content_id(已有)/channel 归因/兴趣 solution_id/沟通记录(lead_events 已有)/需求确认/会议/报价金额/输单原因/下次跟进时间。
opportunities 新表承接商机阶段机。

## 6. Agent 优先级（一期启用）
老何(Commander)/市场研究/内容&营销/销售开发——其余保留通过 agents.status 配置启停。

## 7. 经营驾驶舱指标（替换纯任务计数）
有效潜客/有效商务沟通/演示次数/报价数/成交客户/代运营收入/项目收入/续费收入/投入产出。

## 8. 分阶段实施（映射30天计划）
- W1：solutions 表+矿山样板一条（一页介绍+演示动线）→ 对应30天W1产品盘点
- W2：marketing_briefs + 内容供料 → W2营销样板
- W3：opportunities + 30家企业建档 → W3客户开发
- W4：驾驶舱经营指标 + 验证代运营付费意愿 → W4商业验证

## 9. 兼容承诺
不破坏现有 EDU/ECOM/STUDY/ERP 数据与功能；所有新表走迁移文件；不做复杂ERP/合同系统（三审明确）。
