UPDATE services
SET docker_name = (
  SELECT docker_scan_cache.container_name
  FROM docker_scan_cache
  WHERE docker_scan_cache.endpoint_id = services.docker_endpoint_id
    AND docker_scan_cache.container_id = services.docker_container_id
  ORDER BY docker_scan_cache.scanned_at DESC
  LIMIT 1
)
WHERE (docker_name IS NULL OR docker_name = '')
  AND docker_container_id IS NOT NULL
  AND EXISTS (
    SELECT 1
    FROM docker_scan_cache
    WHERE docker_scan_cache.endpoint_id = services.docker_endpoint_id
      AND docker_scan_cache.container_id = services.docker_container_id
  );
