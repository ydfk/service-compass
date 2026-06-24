CREATE TABLE docker_endpoints (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  endpoint_type TEXT NOT NULL,
  endpoint_url TEXT NOT NULL,
  tls_enabled INTEGER NOT NULL DEFAULT 0,
  tls_ca_secret TEXT,
  tls_cert_secret TEXT,
  tls_key_secret TEXT,
  lan_host TEXT,
  public_host_hint TEXT,
  enabled INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE docker_scan_cache (
  id TEXT PRIMARY KEY,
  endpoint_id TEXT NOT NULL,
  container_id TEXT NOT NULL,
  container_name TEXT NOT NULL,
  image TEXT,
  state TEXT,
  status TEXT,
  labels_json TEXT,
  ports_json TEXT,
  networks_json TEXT,
  candidates_json TEXT,
  scanned_at TEXT NOT NULL,
  FOREIGN KEY(endpoint_id) REFERENCES docker_endpoints(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_docker_scan_endpoint_container
ON docker_scan_cache(endpoint_id, container_id);

