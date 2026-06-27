-- 可选恢复脚本：当迁移 11 曾误删 monitors 时，用 services 信息重建基础监控。
-- 执行前请先备份 SQLite 数据库文件。本脚本不会恢复历史检查记录、关键字、认证或通知规则。

BEGIN;

CREATE TEMP TABLE recovered_http_monitors AS
SELECT
  lower(hex(randomblob(16))) AS id,
  s.id AS service_id,
  s.name || ' HTTP' AS name,
  CASE
    WHEN s.public_url IS NOT NULL AND trim(s.public_url) <> '' THEN 'public'
    ELSE 'local'
  END AS target_url_mode
FROM services s
WHERE
  NOT EXISTS (
    SELECT 1 FROM monitors m
    WHERE m.service_id = s.id AND m.monitor_type IN ('http', 'http_keyword')
  )
  AND (
    (s.public_url IS NOT NULL AND trim(s.public_url) <> '')
    OR (s.local_url IS NOT NULL AND trim(s.local_url) <> '')
  );

INSERT INTO monitors (
  id,
  service_id,
  name,
  monitor_type,
  target_url_mode,
  created_at,
  updated_at
)
SELECT
  id,
  service_id,
  name,
  'http',
  target_url_mode,
  datetime('now'),
  datetime('now')
FROM recovered_http_monitors;

INSERT INTO monitor_states (monitor_id, next_check_at, updated_at)
SELECT id, datetime('now'), datetime('now')
FROM recovered_http_monitors;

CREATE TEMP TABLE recovered_docker_monitors AS
SELECT
  lower(hex(randomblob(16))) AS id,
  s.id AS service_id,
  s.name || ' Docker' AS name
FROM services s
WHERE
  NOT EXISTS (
    SELECT 1 FROM monitors m
    WHERE m.service_id = s.id AND m.monitor_type = 'docker'
  )
  AND s.docker_endpoint_id IS NOT NULL
  AND (
    s.docker_name IS NOT NULL
    OR s.docker_container_id IS NOT NULL
    OR (s.docker_compose_project IS NOT NULL AND s.docker_compose_service IS NOT NULL)
  );

INSERT INTO monitors (
  id,
  service_id,
  name,
  monitor_type,
  target_url_mode,
  created_at,
  updated_at
)
SELECT
  id,
  service_id,
  name,
  'docker',
  'custom',
  datetime('now'),
  datetime('now')
FROM recovered_docker_monitors;

INSERT INTO monitor_states (monitor_id, next_check_at, updated_at)
SELECT id, datetime('now'), datetime('now')
FROM recovered_docker_monitors;

DROP TABLE recovered_http_monitors;
DROP TABLE recovered_docker_monitors;

COMMIT;
