-- 舆情雷达：关键词库（挂业务/目标）→ 扫描信号 → 转选题/线索/脚本
CREATE TABLE IF NOT EXISTS listening_keywords (
  id TEXT PRIMARY KEY,
  business_unit_code TEXT,
  goal_text TEXT,
  keyword TEXT NOT NULL,
  platform TEXT NOT NULL DEFAULT 'news',
  enabled BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS listening_signals (
  id TEXT PRIMARY KEY,
  business_unit_code TEXT,
  keyword TEXT,
  platform TEXT NOT NULL,
  title TEXT NOT NULL,
  url TEXT,
  summary TEXT,
  score INT NOT NULL DEFAULT 5,
  status TEXT NOT NULL DEFAULT 'NEW',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_signals_status ON listening_signals (status);
