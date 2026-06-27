PRAGMA foreign_keys = OFF;

DROP TABLE IF EXISTS services_new;

CREATE TABLE services_new (
  id TEXT PRIMARY KEY,
  group_id TEXT NOT NULL,
  name TEXT NOT NULL,
  description TEXT,
  icon_type TEXT NOT NULL DEFAULT 'auto',
  icon_value TEXT,
  local_url TEXT,
  public_url TEXT,
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

INSERT INTO services_new (
  id,
  group_id,
  name,
  description,
  icon_type,
  icon_value,
  local_url,
  public_url,
  docker_endpoint_id,
  docker_container_id,
  docker_name,
  docker_image,
  docker_compose_project,
  docker_compose_service,
  enabled,
  sort_order,
  created_at,
  updated_at
)
SELECT
  id,
  group_id,
  name,
  description,
  icon_type,
  icon_value,
  local_url,
  public_url,
  docker_endpoint_id,
  docker_container_id,
  docker_name,
  docker_image,
  docker_compose_project,
  docker_compose_service,
  enabled,
  sort_order,
  created_at,
  updated_at
FROM services;

DROP TABLE services;

ALTER TABLE services_new RENAME TO services;

CREATE INDEX idx_services_group_sort ON services(group_id, sort_order, name);

PRAGMA foreign_keys = ON;
