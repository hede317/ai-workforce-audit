use crate::auth::{hash_password, new_id};
use crate::config::Config;
use crate::tools::TOOL_DEFS;
use serde_json::json;
use sqlx::PgPool;

/// 幂等种子：工具注册表 / 12个专业Agent / 4个业务单元 / 管理员 / 初始企业记忆
pub async fn run(pool: &PgPool, cfg: &Config) -> Result<(), anyhow::Error> {
    // 工具注册表
    for t in TOOL_DEFS {
        sqlx::query("INSERT INTO tools (id, code, name, category, description, input_schema, permission_level) VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT (code) DO NOTHING")
            .bind(new_id())
            .bind(t.code)
            .bind(t.name)
            .bind(t.category)
            .bind(t.description)
            .bind(serde_json::from_str::<serde_json::Value>(t.input_schema).unwrap_or_default())
            .bind(t.permission_level)
            .execute(pool)
            .await?;
    }

    // 业务单元（V2.0 规范第七章：培训/电商/留学/企业服务）
    let bus = [
        ("EDU", "国际职业培训", "https://learn.hedeyou.com", "AI职业技能培训与中文职场培训"),
        ("ECOM", "跨境电商", "https://shop.hedeyou.com", "跨境选品与独立站销售"),
        ("STUDY", "留学服务", "https://study.hedeyou.com", "留学评估与申请服务"),
        ("ERP", "企业数字化服务", "https://erp.hedeyou.com", "ERP实施、AI提效与企业数字化咨询"),
    ];
    for (code, name, website, desc) in bus {
        sqlx::query("INSERT INTO business_units (id, code, name, website, description) VALUES ($1,$2,$3,$4,$5) ON CONFLICT (code) DO NOTHING")
            .bind(new_id())
            .bind(code)
            .bind(name)
            .bind(website)
            .bind(desc)
            .execute(pool)
            .await?;
    }

    // 专业 Agent 团队（V2.0 规范第二章，12个）
    let agents: &[(&str, &str, &str, &str, &str, serde_json::Value)] = &[
        ("ceo", "CEO Agent", "总经理", "接收用户经营目标，拆解任务、分配专业Agent、检查质量、汇总结果并向用户汇报。不做具体执行。", "[]", json!([])),
        ("market.research", "市场研究Agent", "研究", "搜集市场信息、竞品、行业趋势与客户需求，识别商业机会，沉淀为可检索的结论。", r#"["web.fetch","memory.search","memory.save"]"#, json!(["web.fetch", "memory.search", "memory.save"])),
        ("content", "内容生产Agent", "内容", "选题研究、文章与SEO内容生产、多语言翻译、社媒文案，产出入库待审。", r#"["content.topics_template","content.draft_article","ai.translate","memory.search","memory.save"]"#, json!(["content.topics_template", "content.draft_article", "ai.translate", "memory.search", "memory.save"])),
        ("video", "视频生产Agent", "视频", "视频脚本整理、数字人/口播视频生成、字幕、封面与格式转换，复用现有出片服务不重复建设。", r#"["video.placeholder","video.render_mpt","memory.search"]"#, json!(["video.placeholder", "video.render_mpt", "memory.search"])),
        ("seo.geo", "SEO/GEO Agent", "搜索", "关键词研究、页面SEO检查、结构化数据建议、GEO内容优化与搜索流量分析。", r#"["seo.check","web.fetch","memory.save"]"#, json!(["seo.check", "web.fetch", "memory.save"])),
        ("social.media", "社媒运营Agent", "社媒", "TikTok/YouTube/Facebook/Instagram/LinkedIn内容运营与发布管理；无授权平台一律走人工发布队列。", r#"["social.package_upload","web.fetch","memory.save"]"#, json!(["social.package_upload", "web.fetch", "memory.save"])),
        ("sales", "销售与客户开发Agent", "销售", "潜在客户研究、线索评分与分类、跟进建议、CRM更新；未经审批不得对外发送邮件或报价。", r#"["crm.create_lead","memory.search","memory.save"]"#, json!(["crm.create_lead", "memory.search", "memory.save"])),
        ("email", "邮件营销Agent", "邮件", "合法联系人的邮件内容生成、分组、发送计划与效果分析；发送动作必须审批。", r#"["memory.search"]"#, json!(["email.send_draft", "memory.search"])),
        ("ecommerce", "电商运营Agent", "电商", "选品分析、竞品价格、商品描述与上架准备、订单与客诉分析；改价/退款必须审批。", r#"["memory.search","crm.create_lead"]"#, json!(["memory.search", "crm.create_lead"])),
        ("education", "培训与留学Agent", "教育", "课程研究、大纲与教学材料、留学项目资料整理、学员咨询分类与招生内容。", r#"["content.draft_article","memory.search"]"#, json!(["content.draft_article", "memory.search"])),
        ("enterprise", "企业服务Agent", "企业服务", "ERP与AI提效市场研究、企业流程诊断资料、方案初稿与项目机会管理。", r#"["content.draft_article","memory.search"]"#, json!(["content.draft_article", "memory.search"])),
        ("data.analyst", "数据分析Agent", "数据", "网站/社媒/SEO/线索转化数据分析，Agent效率与成本核算，每日经营报告。", r#"["data.daily_report","memory.search"]"#, json!(["data.daily_report", "memory.search"])),
    ];
    for (code, name, role, prompt, _tools_str, tools) in agents {
        sqlx::query("INSERT INTO agents (id, code, name, role, responsibilities, system_prompt, tools) VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT (code) DO UPDATE SET tools = EXCLUDED.tools")
            .bind(new_id())
            .bind(code)
            .bind(name)
            .bind(role)
            .bind(prompt)
            .bind(prompt)
            .bind(tools)
            .execute(pool)
            .await?;
    }

    // 管理员（已存在则不动，尊重已改密码）
    let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM users WHERE username=$1").bind(&cfg.admin_username).fetch_optional(pool).await?;
    if exists.is_none() {
        sqlx::query("INSERT INTO users (id, username, password_hash, role) VALUES ($1,$2,$3,'ADMIN')")
            .bind(new_id())
            .bind(&cfg.admin_username)
            .bind(hash_password(&cfg.admin_password)?)
            .execute(pool)
            .await?;
    }

    // 初始企业记忆（用户可随时在 Memory Center 修订）
    let mem_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM memories WHERE kind='ENTERPRISE'").fetch_one(pool).await?;
    if mem_count.0 == 0 {
        let seed_memories = [
            ("公司介绍", "HedeYou：25年跨界经验的独立顾问经营的AI与数字化服务品牌，覆盖企业数字化（ERP/FDE实施）、AI职业技能培训、留学服务、跨境电商四大业务。"),
            ("经营约束", "现金纪律：先修转化断点再扩张工具；所有对外内容必须人工审批；AI生成内容按平台要求标注；广告未埋点不投放。"),
        ];
        for (title, content) in seed_memories {
            sqlx::query("INSERT INTO memories (id, kind, title, content, tags, status) VALUES ($1,'ENTERPRISE',$2,$3,$4,'ACTIVE')")
                .bind(new_id())
                .bind(title)
                .bind(content)
                .bind(json!(["seed"]))
                .execute(pool)
                .await?;
        }
    }
    Ok(())
}
