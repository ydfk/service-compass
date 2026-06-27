ALTER TABLE notification_deliveries RENAME TO notification_deliveries_old;

CREATE TABLE notification_deliveries (
  id TEXT PRIMARY KEY,
  monitor_id TEXT,
  channel_id TEXT,
  event_type TEXT NOT NULL,
  success INTEGER NOT NULL DEFAULT 1,
  request_method TEXT,
  request_url TEXT,
  request_payload TEXT,
  response_status_code INTEGER,
  response_summary TEXT,
  error_message TEXT,
  delivered_at TEXT NOT NULL,
  FOREIGN KEY(channel_id) REFERENCES notification_channels(id) ON DELETE CASCADE
);

INSERT INTO notification_deliveries (
  id,
  monitor_id,
  channel_id,
  event_type,
  success,
  request_method,
  request_url,
  request_payload,
  response_status_code,
  response_summary,
  error_message,
  delivered_at
)
SELECT
  id,
  monitor_id,
  channel_id,
  event_type,
  success,
  request_method,
  request_url,
  request_payload,
  response_status_code,
  response_summary,
  error_message,
  delivered_at
FROM notification_deliveries_old;

DROP TABLE notification_deliveries_old;

CREATE INDEX idx_notification_delivery_cooldown
ON notification_deliveries(monitor_id, channel_id, event_type, success, delivered_at DESC);

CREATE INDEX idx_notification_delivery_time
ON notification_deliveries(delivered_at DESC);
