-- 二审修复：审批不可变快照 + 发布防重库级唯一
ALTER TABLE approvals ADD COLUMN IF NOT EXISTS tool_code TEXT;
ALTER TABLE approvals ADD COLUMN IF NOT EXISTS args_snapshot JSONB;
ALTER TABLE approvals ADD COLUMN IF NOT EXISTS args_hash TEXT;

-- P2-7：内容发布防重的数据库层兜底（并发下应用层检查可被穿透）
CREATE UNIQUE INDEX IF NOT EXISTS uq_active_content_publish
ON publishing_tasks (content_asset_id, platform)
WHERE status IN ('HUMAN_QUEUE', 'DRAFT', 'SCHEDULED', 'PUBLISHING');
