ALTER TABLE backup_config ADD COLUMN aliyun_oss_endpoint TEXT;
ALTER TABLE backup_config ADD COLUMN aliyun_oss_region TEXT;
ALTER TABLE backup_config ADD COLUMN aliyun_oss_bucket TEXT;
ALTER TABLE backup_config ADD COLUMN aliyun_oss_prefix TEXT;
ALTER TABLE backup_config ADD COLUMN aliyun_oss_access_key_id TEXT;
ALTER TABLE backup_config ADD COLUMN aliyun_oss_access_key_secret TEXT;

CREATE TABLE IF NOT EXISTS backup_runs (
    id TEXT PRIMARY KEY,
    run_type TEXT NOT NULL,
    target_type TEXT NOT NULL,
    status TEXT NOT NULL,
    filename TEXT,
    file_size INTEGER,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    http_status_code INTEGER,
    response_summary TEXT,
    error_message TEXT
);

CREATE INDEX IF NOT EXISTS idx_backup_runs_started_at ON backup_runs(started_at DESC);
