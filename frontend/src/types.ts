export type Status = 'up' | 'down' | 'warning' | 'unknown'
export type UrlMode = 'local' | 'public'

export interface Group {
  id: string
  name: string
  description?: string | null
  icon?: string | null
  sort_order: number
  created_at: string
  updated_at: string
}

export interface Service {
  id: string
  group_id: string
  name: string
  description?: string | null
  icon_type: string
  icon_value?: string | null
  icon_url?: string | null
  local_url?: string | null
  public_url?: string | null
  preferred_url_mode: UrlMode
  docker_name?: string | null
  docker_image?: string | null
  docker_endpoint_id?: string | null
  docker_container_id?: string | null
  docker_compose_project?: string | null
  docker_compose_service?: string | null
  enabled: boolean
  sort_order: number
  status?: Status
  last_latency_ms?: number | null
  last_error?: string | null
  monitor_tracks?: MonitorTrack[]
}

export interface MonitorTrack {
  id: string
  monitor_type: Monitor['monitor_type']
  status: Status
  uptime_percent?: number | null
  segments: Status[]
  segment_details?: StatusPoint[]
  last_checked_at?: string | null
  last_latency_ms?: number | null
}

export interface StatusPoint {
  status: Status
  checked_at?: string | null
  latency_ms?: number | null
  status_code?: number | null
  message?: string | null
}

export interface DashboardGroup extends Group {
  services: Service[]
}

export interface ServiceInput {
  group_id?: string | null
  name: string
  description?: string | null
  icon_type: string
  icon_value?: string | null
  local_url?: string | null
  public_url?: string | null
  preferred_url_mode: UrlMode
  enabled: boolean
  sort_order: number
  docker_endpoint_id?: string | null
  docker_container_id?: string | null
  docker_name?: string | null
  docker_image?: string | null
  docker_compose_project?: string | null
  docker_compose_service?: string | null
  create_monitor: boolean
  cert_expiry_notify: boolean
  monitor_type?: 'http'
  monitor_target_url_mode?: 'local' | 'public'
  monitor?: MonitorInput | null
}

export interface Monitor {
  id: string
  service_id?: string | null
  name: string
  monitor_type: 'http' | 'http_keyword' | 'dns' | 'cert' | 'docker'
  target_url?: string | null
  target_url_mode: 'custom' | UrlMode
  method: string
  expected_status_min: number
  expected_status_max: number
  keyword?: string | null
  interval_sec: number
  timeout_sec: number
  retries: number
  retry_interval_sec: number
  follow_redirects: boolean
  tls_verify: boolean
  request_body_type: 'json' | 'form'
  has_request_body?: boolean
  has_request_headers?: boolean
  request_body?: string | null
  request_headers?: string | null
  auth_type: 'none' | 'basic'
  auth_username?: string | null
  has_auth_password?: boolean
  auth_password?: string | null
  domain?: string | null
  record_type: string
  expected_value?: string | null
  cert_port: number
  cert_warning_days: number
  cert_critical_days: number
  enabled: boolean
  current_status: Status
  last_checked_at?: string | null
  last_latency_ms?: number | null
  last_error?: string | null
  last_extra_json?: string | null
  recent_statuses: Status[]
  recent_checks: MonitorCheck[]
  notify_enabled: boolean
  notification_channel_ids: string[]
  notify_on_down: boolean
  notify_on_recovery: boolean
  notify_on_warning: boolean
  notification_cooldown_sec: number
}

export type MonitorInput = Omit<
  Monitor,
  | 'id'
  | 'has_auth_password'
  | 'has_request_body'
  | 'has_request_headers'
  | 'current_status'
  | 'last_checked_at'
  | 'last_latency_ms'
  | 'last_error'
  | 'last_extra_json'
>

export interface MonitorCheck {
  id: string
  monitor_id: string
  status: Status
  latency_ms?: number | null
  status_code?: number | null
  error_message?: string | null
  checked_at: string
  extra_json?: string | null
}

export interface DockerEndpoint {
  id: string
  name: string
  endpoint_type: 'local_socket' | 'remote_tcp'
  endpoint_url: string
  tls_enabled: boolean
  has_tls_ca: boolean
  has_tls_cert: boolean
  has_tls_key: boolean
  lan_host?: string | null
  public_host_hint?: string | null
  enabled: boolean
}

export interface DockerEndpointInput {
  name: string
  endpoint_type: 'local_socket' | 'remote_tcp'
  endpoint_url: string
  tls_enabled: boolean
  tls_ca?: string | null
  tls_cert?: string | null
  tls_key?: string | null
  lan_host?: string | null
  public_host_hint?: string | null
  enabled: boolean
}

export interface DockerCandidate {
  endpoint_id: string
  container_id: string
  container_name: string
  image?: string | null
  state?: string | null
  status?: string | null
  suggested_name: string
  suggested_group: string
  suggested_icon: string
  local_url?: string | null
  public_url?: string | null
  compose_project?: string | null
  compose_service?: string | null
  ports: string[]
}

export interface NotificationChannel {
  id: string
  name: string
  channel_type: 'bark' | 'webhook' | 'synology_chat'
  enabled: boolean
  configured: boolean
  config: Record<string, unknown>
}

export interface NotificationRule {
  id: string
  monitor_id?: string | null
  channel_id: string
  notify_on_down: boolean
  notify_on_recovery: boolean
  notify_on_warning: boolean
  cooldown_sec: number
  enabled: boolean
  created_at: string
  updated_at: string
}

export type NotificationRuleInput = Omit<NotificationRule, 'id' | 'created_at' | 'updated_at'>
