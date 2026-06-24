CREATE TABLE notification_channels (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  channel_type TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  config_secret TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE notification_rules (
  id TEXT PRIMARY KEY,
  monitor_id TEXT,
  channel_id TEXT NOT NULL,
  notify_on_down INTEGER NOT NULL DEFAULT 1,
  notify_on_recovery INTEGER NOT NULL DEFAULT 1,
  notify_on_warning INTEGER NOT NULL DEFAULT 1,
  cooldown_sec INTEGER NOT NULL DEFAULT 300,
  enabled INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(monitor_id) REFERENCES monitors(id) ON DELETE CASCADE,
  FOREIGN KEY(channel_id) REFERENCES notification_channels(id) ON DELETE CASCADE
);

CREATE TABLE notification_deliveries (
  id TEXT PRIMARY KEY,
  monitor_id TEXT NOT NULL,
  channel_id TEXT NOT NULL,
  event_type TEXT NOT NULL,
  delivered_at TEXT NOT NULL,
  FOREIGN KEY(monitor_id) REFERENCES monitors(id) ON DELETE CASCADE,
  FOREIGN KEY(channel_id) REFERENCES notification_channels(id) ON DELETE CASCADE
);

CREATE INDEX idx_notification_delivery_cooldown
ON notification_deliveries(monitor_id, channel_id, event_type, delivered_at DESC);

