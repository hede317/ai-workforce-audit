-- Sprint 3：选品中心（ECOM）——商品上架/改价等敏感动作需权限门
CREATE TABLE IF NOT EXISTS product_candidates (
  id TEXT PRIMARY KEY,
  business_unit_code TEXT,
  title TEXT NOT NULL,
  source_url TEXT,
  price DOUBLE PRECISION NOT NULL DEFAULT 0,
  est_cost DOUBLE PRECISION NOT NULL DEFAULT 0,
  est_margin_pct DOUBLE PRECISION NOT NULL DEFAULT 0,
  demand_score INT NOT NULL DEFAULT 0,
  competition_score INT NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'PENDING',
  decision_note TEXT,
  source_task_id TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_products_status ON product_candidates (status);
