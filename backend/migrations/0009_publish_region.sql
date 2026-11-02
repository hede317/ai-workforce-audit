-- 0009: 发布区域分区（国内 / 海外）+ 自动发布留痕
--
-- 背景：原 publishing_tasks 只有 platform 字段，无法区分「国内平台」与「海外平台」。
-- 国内平台（抖音/B站/快手/小红书/视频号）走 social-auto-upload 的浏览器自动化；
-- 海外平台（YouTube/TikTok）走同一套 CLI 但需要代理。
-- 两者失败模式、账号体系、合规要求都不同，必须能分开统计与分别开关。

ALTER TABLE publishing_tasks
  ADD COLUMN IF NOT EXISTS region TEXT NOT NULL DEFAULT 'DOMESTIC';

-- 自动发布专用字段：区分「人工队列」与「机器直发」，并留下子进程证据
ALTER TABLE publishing_tasks
  ADD COLUMN IF NOT EXISTS mode TEXT NOT NULL DEFAULT 'HUMAN_QUEUE';

ALTER TABLE publishing_tasks
  ADD COLUMN IF NOT EXISTS started_at TIMESTAMPTZ;

ALTER TABLE publishing_tasks
  ADD COLUMN IF NOT EXISTS finished_at TIMESTAMPTZ;

ALTER TABLE publishing_tasks
  ADD COLUMN IF NOT EXISTS exit_log TEXT;

-- 只允许已知区域值，防止后续代码写错拼写
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint WHERE conname = 'publishing_tasks_region_check'
  ) THEN
    ALTER TABLE publishing_tasks
      ADD CONSTRAINT publishing_tasks_region_check
      CHECK (region IN ('DOMESTIC','OVERSEAS'));
  END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_publishing_tasks_region_status
  ON publishing_tasks (region, status, created_at DESC);

-- 同一条内容在同一平台同一账号下，同时只允许一个进行中的自动发布任务
CREATE UNIQUE INDEX IF NOT EXISTS uniq_publishing_active_auto
  ON publishing_tasks (content_asset_id, platform, COALESCE(account, ''))
  WHERE status IN ('PUBLISHING') AND mode = 'AUTO';
