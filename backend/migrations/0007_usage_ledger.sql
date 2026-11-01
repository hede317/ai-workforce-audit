-- T-003：AI 调用台账 + 日预算护栏
CREATE TABLE IF NOT EXISTS usage_ledger (
  id TEXT PRIMARY KEY,
  task_id TEXT NULL,
  tokens BIGINT NOT NULL DEFAULT 0,
  cost_usd DOUBLE PRECISION NOT NULL DEFAULT 0,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_usage_created ON usage_ledger (created_at);
