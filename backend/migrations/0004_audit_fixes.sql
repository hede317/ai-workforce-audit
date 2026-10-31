-- Sprint 4（审计修复）：幂等唯一键纠正 / 线索邮箱唯一 / 缺失索引 / 数据清洗
-- 1. 幂等键作用域从全局改为 (endpoint, idem_key)，避免跨端点键冲突
ALTER TABLE idempotency_keys DROP CONSTRAINT IF EXISTS idempotency_keys_idem_key_key;
CREATE UNIQUE INDEX IF NOT EXISTS uq_idem_endpoint_key ON idempotency_keys (endpoint, idem_key);

-- 2. 线索邮箱唯一（大小写/空格规范化后），空串清空为 NULL
UPDATE leads SET email = NULL WHERE email IS NOT NULL AND trim(email) = '';
CREATE UNIQUE INDEX IF NOT EXISTS uq_leads_email ON leads (lower(btrim(email))) WHERE email IS NOT NULL AND email <> '';

-- 3. 高频查询缺失索引
CREATE INDEX IF NOT EXISTS idx_approvals_task ON approvals (task_id);
CREATE INDEX IF NOT EXISTS idx_pubresults_task ON publishing_results (publishing_task_id);
CREATE INDEX IF NOT EXISTS idx_leadevents_lead ON lead_events (lead_id);

-- 注：memories.embedding 为无固定维度 vector，pgvector 无法建 HNSW/IVF 索引；
-- 已改用库内 <=> 算子暴力余弦（仍优于拉回应用层），待固定 embedding 模型后可 ALTER 列维度并加索引。
