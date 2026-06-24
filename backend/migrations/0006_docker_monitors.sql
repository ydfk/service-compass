INSERT INTO monitors (
  id, service_id, name, monitor_type, target_url_mode, interval_sec, enabled, created_at, updated_at
)
SELECT
  lower(hex(randomblob(16))),
  s.id,
  s.name || ' Docker',
  'docker',
  'custom',
  60,
  1,
  strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
  strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
FROM services s
WHERE s.docker_endpoint_id IS NOT NULL
  AND s.docker_container_id IS NOT NULL
  AND NOT EXISTS (
    SELECT 1 FROM monitors m WHERE m.service_id = s.id AND m.monitor_type = 'docker'
  );

INSERT INTO monitor_states (monitor_id, next_check_at, updated_at)
SELECT
  m.id,
  strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
  strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
FROM monitors m
WHERE m.monitor_type = 'docker'
  AND NOT EXISTS (SELECT 1 FROM monitor_states s WHERE s.monitor_id = m.id);
