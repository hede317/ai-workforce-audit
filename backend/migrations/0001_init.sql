-- HedeYou AI Workforce — PostgreSQL 初始化（PG 16+ / pgvector 就绪）
CREATE TABLE IF NOT EXISTS users (
  id TEXT PRIMARY KEY,
  username TEXT NOT NULL UNIQUE,
  password_hash TEXT NOT NULL,
  role TEXT NOT NULL DEFAULT 'OPERATOR',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS business_units (
  id TEXT PRIMARY KEY,
  code TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  website TEXT NOT NULL DEFAULT '',
  description TEXT,
  default_language TEXT NOT NULL DEFAULT 'zh',
  configuration JSONB,
  status TEXT NOT NULL DEFAULT 'ACTIVE',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS agents (
  id TEXT PRIMARY KEY,
  code TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  role TEXT NOT NULL,
  responsibilities TEXT,
  system_prompt TEXT,
  model TEXT,
  tools JSONB,
  max_depth INT NOT NULL DEFAULT 2,
  max_tool_calls INT NOT NULL DEFAULT 12,
  max_tokens INT NOT NULL DEFAULT 8000,
  max_seconds INT NOT NULL DEFAULT 120,
  status TEXT NOT NULL DEFAULT 'ACTIVE',
  configuration JSONB,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS tools (
  id TEXT PRIMARY KEY,
  code TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  category TEXT NOT NULL,
  description TEXT,
  input_schema JSONB,
  permission_level TEXT NOT NULL DEFAULT 'AUTO',
  timeout_seconds INT NOT NULL DEFAULT 30,
  status TEXT NOT NULL DEFAULT 'ACTIVE',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS tasks (
  id TEXT PRIMARY KEY,
  parent_id TEXT,
  root_id TEXT,
  title TEXT NOT NULL,
  description TEXT,
  goal TEXT,
  business_unit_code TEXT,
  agent_code TEXT,
  status TEXT NOT NULL DEFAULT 'Created',
  priority INT NOT NULL DEFAULT 5,
  depth INT NOT NULL DEFAULT 0,
  tool_hint TEXT,
  args JSONB,
  plan JSONB,
  result JSONB,
  error TEXT,
  requires_approval BOOLEAN NOT NULL DEFAULT FALSE,
  max_tool_calls INT NOT NULL DEFAULT 12,
  max_tokens INT NOT NULL DEFAULT 8000,
  max_seconds INT NOT NULL DEFAULT 120,
  tool_calls_used INT NOT NULL DEFAULT 0,
  tokens_used INT NOT NULL DEFAULT 0,
  started_at TIMESTAMPTZ,
  finished_at TIMESTAMPTZ,
  created_by TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_tasks_root ON tasks (root_id);
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks (status);

CREATE TABLE IF NOT EXISTS task_events (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL,
  event TEXT NOT NULL,
  payload JSONB,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_events_task ON task_events (task_id);

CREATE TABLE IF NOT EXISTS approvals (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL,
  action TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'PENDING',
  requested_by TEXT,
  decided_by TEXT,
  note TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  decided_at TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_approvals_status ON approvals (status);

CREATE TABLE IF NOT EXISTS memories (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL,
  title TEXT NOT NULL,
  content TEXT NOT NULL,
  tags JSONB,
  source_task_id TEXT,
  status TEXT NOT NULL DEFAULT 'ACTIVE',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_memories_kind ON memories (kind);

CREATE TABLE IF NOT EXISTS content_topics (
  id TEXT PRIMARY KEY,
  business_unit_code TEXT,
  title TEXT NOT NULL,
  keywords JSONB,
  audience TEXT,
  market TEXT,
  language TEXT NOT NULL DEFAULT 'zh',
  priority INT NOT NULL DEFAULT 5,
  status TEXT NOT NULL DEFAULT 'NEW',
  source_task_id TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS content_assets (
  id TEXT PRIMARY KEY,
  topic_id TEXT,
  business_unit_code TEXT,
  content_type TEXT NOT NULL DEFAULT 'article',
  title TEXT NOT NULL,
  body TEXT,
  language TEXT NOT NULL DEFAULT 'zh',
  status TEXT NOT NULL DEFAULT 'DRAFT',
  seo_metadata JSONB,
  source_task_id TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS publishing_tasks (
  id TEXT PRIMARY KEY,
  content_asset_id TEXT,
  platform TEXT NOT NULL,
  account TEXT,
  scheduled_at TIMESTAMPTZ,
  status TEXT NOT NULL DEFAULT 'DRAFT',
  payload JSONB,
  error TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS publishing_results (
  id TEXT PRIMARY KEY,
  publishing_task_id TEXT NOT NULL,
  platform_post_id TEXT,
  published_url TEXT,
  published_at TIMESTAMPTZ,
  metrics JSONB,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS leads (
  id TEXT PRIMARY KEY,
  business_unit_code TEXT,
  name TEXT,
  email TEXT,
  phone TEXT,
  country TEXT,
  source_channel TEXT,
  source_content_id TEXT,
  consent_status TEXT NOT NULL DEFAULT 'UNKNOWN',
  lead_status TEXT NOT NULL DEFAULT 'NEW',
  intent_level TEXT,
  summary TEXT,
  metadata JSONB,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_leads_status ON leads (lead_status);

CREATE TABLE IF NOT EXISTS lead_events (
  id TEXT PRIMARY KEY,
  lead_id TEXT NOT NULL,
  event_type TEXT NOT NULL,
  note TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS campaigns (
  id TEXT PRIMARY KEY,
  business_unit_code TEXT,
  name TEXT NOT NULL,
  channel TEXT,
  budget DOUBLE PRECISION NOT NULL DEFAULT 0,
  spend DOUBLE PRECISION NOT NULL DEFAULT 0,
  start_date DATE,
  end_date DATE,
  utm JSONB,
  status TEXT NOT NULL DEFAULT 'DRAFT',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS workflow_executions (
  id TEXT PRIMARY KEY,
  workflow_code TEXT NOT NULL,
  external_id TEXT,
  status TEXT NOT NULL DEFAULT 'RUNNING',
  started_at TIMESTAMPTZ,
  finished_at TIMESTAMPTZ,
  error TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS audit_logs (
  id TEXT PRIMARY KEY,
  user_id TEXT,
  action TEXT NOT NULL,
  entity TEXT,
  entity_id TEXT,
  detail JSONB,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS idempotency_keys (
  id TEXT PRIMARY KEY,
  idem_key TEXT NOT NULL UNIQUE,
  endpoint TEXT NOT NULL,
  response JSONB,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
