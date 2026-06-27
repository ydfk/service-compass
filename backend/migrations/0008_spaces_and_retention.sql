CREATE TABLE spaces (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

INSERT INTO spaces (id, name, description, sort_order, created_at, updated_at)
VALUES (
  '00000000-0000-0000-0000-000000000001',
  '默认空间',
  NULL,
  0,
  datetime('now'),
  datetime('now')
);

ALTER TABLE groups
ADD COLUMN space_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000001';

INSERT OR IGNORE INTO settings (key, value, updated_at)
VALUES
  ('retention_days', '30', datetime('now')),
  ('log_retention_days', '30', datetime('now'));
