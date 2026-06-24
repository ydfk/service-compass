CREATE TABLE monitors (
  id TEXT PRIMARY KEY,
  service_id TEXT,
  name TEXT NOT NULL,
  monitor_type TEXT NOT NULL,
  target_url TEXT,
  target_url_mode TEXT NOT NULL DEFAULT 'custom',
  method TEXT NOT NULL DEFAULT 'GET',
  expected_status_min INTEGER NOT NULL DEFAULT 200,
  expected_status_max INTEGER NOT NULL DEFAULT 399,
  keyword TEXT,
  interval_sec INTEGER NOT NULL DEFAULT 60,
  timeout_sec INTEGER NOT NULL DEFAULT 10,
  retries INTEGER NOT NULL DEFAULT 2,
  retry_interval_sec INTEGER NOT NULL DEFAULT 5,
  follow_redirects INTEGER NOT NULL DEFAULT 1,
  tls_verify INTEGER NOT NULL DEFAULT 1,
  auth_type TEXT NOT NULL DEFAULT 'none',
  auth_username TEXT,
  auth_password_secret TEXT,
  domain TEXT,
  record_type TEXT NOT NULL DEFAULT 'A',
  expected_value TEXT,
  cert_port INTEGER NOT NULL DEFAULT 443,
  cert_warning_days INTEGER NOT NULL DEFAULT 30,
  cert_critical_days INTEGER NOT NULL DEFAULT 7,
  enabled INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(service_id) REFERENCES services(id) ON DELETE CASCADE
);

CREATE TABLE monitor_checks (
  id TEXT PRIMARY KEY,
  monitor_id TEXT NOT NULL,
  status TEXT NOT NULL,
  latency_ms INTEGER,
  status_code INTEGER,
  error_message TEXT,
  checked_at TEXT NOT NULL,
  extra_json TEXT,
  FOREIGN KEY(monitor_id) REFERENCES monitors(id) ON DELETE CASCADE
);

CREATE INDEX idx_monitor_checks_monitor_time
ON monitor_checks(monitor_id, checked_at DESC);

CREATE TABLE monitor_states (
  monitor_id TEXT PRIMARY KEY,
  current_status TEXT NOT NULL DEFAULT 'unknown',
  previous_status TEXT,
  consecutive_failures INTEGER NOT NULL DEFAULT 0,
  last_checked_at TEXT,
  last_up_at TEXT,
  last_down_at TEXT,
  last_latency_ms INTEGER,
  last_error TEXT,
  next_check_at TEXT,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(monitor_id) REFERENCES monitors(id) ON DELETE CASCADE
);

CREATE INDEX idx_monitor_states_due ON monitor_states(next_check_at);

