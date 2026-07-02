import type { Monitor, MonitorInput, Service, ServiceInput } from '../types'

export const UNGROUPED_ID = '00000000-0000-0000-0000-000000000000'

export function emptyService(): ServiceInput {
  return {
    group_id: '',
    name: '',
    description: '',
    icon_type: 'auto',
    icon_value: '',
    local_url: '',
    public_url: '',
    enabled: true,
    sort_order: 0,
    create_monitor: true,
    cert_expiry_notify: false,
    monitor_type: 'http_keyword',
    monitor_target_url_mode: 'public',
    status_notify_enabled: false,
    status_notification_channel_ids: [],
  }
}

export function emptyHttpMonitor(): MonitorInput {
  return {
    service_id: null,
    name: '',
    monitor_type: 'http_keyword',
    target_url: '',
    target_url_mode: 'public',
    method: 'GET',
    expected_status_min: 200,
    expected_status_max: 399,
    keyword: '',
    interval_sec: 60,
    timeout_sec: 10,
    retries: 2,
    retry_interval_sec: 5,
    follow_redirects: true,
    tls_verify: true,
    request_body_type: 'json',
    request_body: '',
    request_headers: '',
    auth_type: 'none',
    auth_username: '',
    auth_password: '',
    domain: '',
    record_type: 'A',
    expected_value: '',
    cert_port: 443,
    cert_warning_days: 30,
    cert_critical_days: 7,
    enabled: true,
    notify_enabled: false,
    notification_channel_ids: [],
    notify_on_down: true,
    notify_on_recovery: true,
    notify_on_warning: true,
    notification_cooldown_sec: 300,
  }
}

export function monitorToInput(monitor: Monitor): MonitorInput {
  return {
    service_id: monitor.service_id,
    name: monitor.name,
    monitor_type: monitor.monitor_type,
    target_url: monitor.target_url,
    target_url_mode: monitor.target_url_mode,
    method: monitor.method,
    expected_status_min: monitor.expected_status_min,
    expected_status_max: monitor.expected_status_max,
    keyword: monitor.keyword,
    interval_sec: monitor.interval_sec,
    timeout_sec: monitor.timeout_sec,
    retries: monitor.retries,
    retry_interval_sec: monitor.retry_interval_sec,
    follow_redirects: monitor.follow_redirects,
    tls_verify: monitor.tls_verify,
    request_body_type: monitor.request_body_type,
    request_body: '',
    request_headers: '',
    auth_type: monitor.auth_type,
    auth_username: monitor.auth_username,
    auth_password: '',
    domain: monitor.domain,
    record_type: monitor.record_type,
    expected_value: monitor.expected_value,
    cert_port: monitor.cert_port,
    cert_warning_days: monitor.cert_warning_days,
    cert_critical_days: monitor.cert_critical_days,
    enabled: monitor.enabled,
    notify_enabled: monitor.notify_enabled,
    notification_channel_ids: monitor.notification_channel_ids,
    notify_on_down: monitor.notify_on_down,
    notify_on_recovery: monitor.notify_on_recovery,
    notify_on_warning: monitor.notify_on_warning,
    notification_cooldown_sec: monitor.notification_cooldown_sec,
  }
}

export function serviceToInput(
  service: Service,
  monitor?: Monitor,
  certMonitor?: Monitor,
  dockerMonitor?: Monitor,
): ServiceInput {
  const notifySource = monitor ?? dockerMonitor ?? certMonitor
  return {
    group_id: service.group_id === UNGROUPED_ID ? '' : service.group_id,
    name: service.name,
    description: service.description,
    icon_type: service.icon_type,
    icon_value: service.icon_value,
    local_url: service.local_url,
    public_url: service.public_url,
    enabled: service.enabled,
    sort_order: service.sort_order,
    docker_endpoint_id: service.docker_endpoint_id,
    docker_container_id: service.docker_container_id,
    docker_name: service.docker_name,
    docker_image: service.docker_image,
    docker_compose_project: service.docker_compose_project,
    docker_compose_service: service.docker_compose_service,
    create_monitor: Boolean(monitor?.enabled),
    cert_expiry_notify: Boolean(certMonitor?.enabled),
    monitor_type: primaryMonitorType(monitor),
    monitor_target_url_mode: monitor?.target_url_mode === 'local' ? 'local' : 'public',
    status_notify_enabled: notifySource?.notify_enabled ?? false,
    status_notification_channel_ids: notifySource?.notification_channel_ids ?? [],
  }
}

export function serviceHttpMonitor(monitors: Monitor[], serviceId: string) {
  return monitors.find(
    (item) =>
      item.service_id === serviceId &&
      ['http', 'http_keyword', 'postgres'].includes(item.monitor_type),
  )
}

export function serviceCertMonitor(monitors: Monitor[], serviceId: string) {
  return monitors.find((item) => item.service_id === serviceId && item.monitor_type === 'cert')
}

export function serviceDockerMonitor(monitors: Monitor[], serviceId: string) {
  return monitors.find((item) => item.service_id === serviceId && item.monitor_type === 'docker')
}

export function cleanServiceInput(input: ServiceInput): ServiceInput {
  return {
    ...input,
    local_url: cleanUrl(input.local_url),
    public_url: cleanUrl(input.public_url),
  }
}

export function validateServiceDraft(input: ServiceInput, monitor: MonitorInput): string | null {
  if (!input.name.trim()) return '请填写服务名称'
  const dockerEnabled = Boolean(input.docker_endpoint_id)
  if (
    dockerEnabled &&
    !input.docker_container_id &&
    !input.docker_name &&
    !(input.docker_compose_project && input.docker_compose_service)
  ) {
    return '关联 Docker 时请选择具体容器'
  }
  if (input.create_monitor) {
    if (monitor.monitor_type === 'postgres') {
      if (!monitor.target_url?.trim()) return 'PostgreSQL 监控需要填写主机'
      if (!monitor.cert_port || monitor.cert_port < 1 || monitor.cert_port > 65535) {
        return 'PostgreSQL 端口无效'
      }
      if (!monitor.domain?.trim()) return 'PostgreSQL 监控需要填写数据库名'
      if (!monitor.auth_username?.trim()) return 'PostgreSQL 监控需要填写用户名'
      if (!isReadonlyPostgresQuery(monitor.expected_value || 'SELECT 1')) {
        return 'PostgreSQL 检查 SQL 仅支持 SELECT、SHOW 或 WITH 查询'
      }
    }
    if (monitor.monitor_type === 'http_keyword' && !monitor.keyword?.trim()) {
      return 'HTTP 关键字监控需要填写响应关键字，或切换为普通 HTTP'
    }
    const isHttp = ['http', 'http_keyword'].includes(monitor.monitor_type)
    if (isHttp && monitor.target_url_mode === 'custom' && !monitor.target_url?.trim()) {
      return '自定义监控地址不能为空'
    }
    if (
      isHttp &&
      ['local', 'public'].includes(monitor.target_url_mode) &&
      !input.local_url?.trim() &&
      !input.public_url?.trim()
    ) {
      return '监控使用服务地址时，内网地址或外网地址至少填写一个'
    }
    if (isHttp && monitor.method === 'POST' && monitor.request_headers?.trim()) {
      try {
        JSON.parse(monitor.request_headers)
      } catch {
        return '请求头必须是合法 JSON'
      }
    }
  }
  if (input.status_notify_enabled && !input.status_notification_channel_ids?.length) {
    return '开启状态通知时必须选择通知通道'
  }
  return null
}

function cleanUrl(value?: string | null) {
  const trimmed = value?.trim() || ''
  if (!trimmed || trimmed === 'http://' || trimmed === 'https://') return ''
  return trimmed
}

function primaryMonitorType(monitor?: Monitor): ServiceInput['monitor_type'] {
  if (monitor?.monitor_type === 'postgres') return 'postgres'
  if (monitor?.monitor_type === 'http') return 'http'
  return 'http_keyword'
}

function isReadonlyPostgresQuery(query: string) {
  const normalized = query.trimStart().toLowerCase()
  return (
    normalized === 'select' ||
    normalized.startsWith('select ') ||
    normalized === 'show' ||
    normalized.startsWith('show ') ||
    normalized === 'with' ||
    normalized.startsWith('with ')
  )
}
