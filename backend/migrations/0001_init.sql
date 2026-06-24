PRAGMA foreign_keys = ON;

CREATE TABLE groups (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  icon TEXT,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE services (
  id TEXT PRIMARY KEY,
  group_id TEXT NOT NULL,
  name TEXT NOT NULL,
  description TEXT,
  icon_type TEXT NOT NULL DEFAULT 'auto',
  icon_value TEXT,
  local_url TEXT,
  public_url TEXT,
  preferred_url_mode TEXT NOT NULL DEFAULT 'local',
  docker_endpoint_id TEXT,
  docker_container_id TEXT,
  docker_name TEXT,
  docker_image TEXT,
  docker_compose_project TEXT,
  docker_compose_service TEXT,
  enabled INTEGER NOT NULL DEFAULT 1,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(group_id) REFERENCES groups(id) ON DELETE RESTRICT
);

CREATE INDEX idx_services_group_sort ON services(group_id, sort_order, name);

CREATE TABLE settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

