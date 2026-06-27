INSERT OR IGNORE INTO settings (key, value, updated_at)
VALUES ('dashboard_refresh_interval_sec', '30', datetime('now'));

CREATE TABLE IF NOT EXISTS backup_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    enabled BOOLEAN NOT NULL DEFAULT 0,
    schedule_time TEXT NOT NULL DEFAULT '03:00',
    target_type TEXT NOT NULL DEFAULT 'local',
    local_dir TEXT,
    webdav_url TEXT,
    webdav_username TEXT,
    webdav_password_secret TEXT,
    retention_count INTEGER NOT NULL DEFAULT 7,
    last_run_at TEXT,
    updated_at TEXT NOT NULL
);

INSERT OR IGNORE INTO backup_config (
    id, enabled, schedule_time, target_type, retention_count, updated_at
) VALUES (1, 0, '03:00', 'local', 7, datetime('now'));
