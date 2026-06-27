<script setup lang="ts">
import { Bell, History, PlayerPlay } from '@vicons/tabler'
import {
  NButton,
  NCard,
  NDataTable,
  NDescriptions,
  NDescriptionsItem,
  NDrawer,
  NDrawerContent,
  NForm,
  NFormItem,
  NIcon,
  NInput,
  NModal,
  NSelect,
  NSpace,
  NStatistic,
  NSwitch,
  NTag,
  useMessage,
  type DataTableColumns,
  type PaginationProps,
} from 'naive-ui'
import { computed, h, onMounted, reactive, ref } from 'vue'
import { groupsApi } from '../api/groups'
import { monitorsApi } from '../api/monitors'
import { notificationsApi } from '../api/notifications'
import { servicesApi } from '../api/services'
import StatusBadge from '../components/StatusBadge.vue'
import StatusStrip from '../components/StatusStrip.vue'
import type {
  Group,
  Monitor,
  MonitorCheck,
  MonitorInput,
  NotificationChannel,
  Service,
  Space,
  Status,
  StatusPoint,
} from '../types'
import { monitorToInput } from '../utils/serviceForms'

const monitors = ref<Monitor[]>([])
const services = ref<Service[]>([])
const spaces = ref<Space[]>([])
const groups = ref<Group[]>([])
const channels = ref<NotificationChannel[]>([])
const checks = ref<MonitorCheck[]>([])
const selectedMonitor = ref<Monitor | null>(null)
const notificationMonitor = ref<Monitor | null>(null)
const notificationForm = ref<MonitorInput>(emptyMonitor())
const historyDrawer = ref(false)
const notificationModal = ref(false)
const loading = ref(false)
const selectedGroupId = ref('')
const selectedServiceId = ref('')
const selectedSpaceId = ref('')
const selectedStatus = ref<Status | ''>('')
const search = ref('')
const message = useMessage()

const serviceById = computed(() => new Map(services.value.map((item) => [item.id, item])))
const groupById = computed(() => new Map(groups.value.map((item) => [item.id, item])))
const scopedGroups = computed(() =>
  selectedSpaceId.value
    ? groups.value.filter((group) => group.space_id === selectedSpaceId.value)
    : groups.value,
)
const spaceOptions = computed(() => [
  { label: '全部空间', value: '' },
  ...spaces.value.map((item) => ({ label: item.name, value: item.id })),
])
const groupOptions = computed(() => [
  { label: '全部分组', value: '' },
  ...scopedGroups.value.map((item) => ({ label: groupLabel(item), value: item.id })),
])
const serviceOptions = computed(() => {
  const filtered = services.value.filter((service) => {
    if (selectedGroupId.value) return service.group_id === selectedGroupId.value
    if (selectedSpaceId.value) return serviceSpaceId(service) === selectedSpaceId.value
    return true
  })
  return [
    { label: '全部服务', value: '' },
    ...filtered.map((item) => ({ label: item.name, value: item.id })),
  ]
})
const statusOptions = [
  { label: '全部状态', value: '' },
  { label: '在线', value: 'up' },
  { label: '异常', value: 'down' },
  { label: '告警', value: 'warning' },
  { label: '未知', value: 'unknown' },
]
const channelOptions = computed(() =>
  channels.value.map((item) => ({ label: item.name, value: item.id })),
)
const filteredMonitors = computed(() =>
  monitors.value
    .filter((monitor) => {
      if (selectedServiceId.value) return monitor.service_id === selectedServiceId.value
      const service = monitor.service_id ? serviceById.value.get(monitor.service_id) : null
      if (selectedGroupId.value) return service?.group_id === selectedGroupId.value
      if (selectedSpaceId.value) return service && serviceSpaceId(service) === selectedSpaceId.value
      return true
    })
    .filter((monitor) => {
      if (!selectedStatus.value) return true
      return monitor.current_status === selectedStatus.value
    })
    .filter((monitor) => {
      const keyword = search.value.trim().toLowerCase()
      if (!keyword) return true
      const service = monitor.service_id ? serviceById.value.get(monitor.service_id) : null
      const group = service?.group_id ? groupById.value.get(service.group_id) : null
      return searchableText(
        monitor.name,
        monitor.monitor_type,
        monitor.target_url,
        monitor.domain,
        monitor.last_error,
        service?.name,
        service?.docker_name,
        service?.docker_image,
        group?.name,
      ).includes(keyword)
    })
    .sort(
      (left, right) =>
        serviceScope(left).localeCompare(serviceScope(right)) ||
        left.name.localeCompare(right.name),
    ),
)
const stats = computed(() => ({
  total: filteredMonitors.value.length,
  up: filteredMonitors.value.filter((item) => item.current_status === 'up').length,
  down: filteredMonitors.value.filter((item) => item.current_status === 'down').length,
  warning: filteredMonitors.value.filter((item) => item.current_status === 'warning').length,
}))

const columns: DataTableColumns<Monitor> = [
  { title: '状态', key: 'status', render: (row) => h(StatusBadge, { status: row.current_status }) },
  { title: '监控', key: 'name', render: (row) => h('strong', row.name) },
  {
    title: '服务 / 分组',
    key: 'service',
    ellipsis: { tooltip: true },
    render: (row) => serviceCell(row),
  },
  {
    title: '类型',
    key: 'monitor_type',
    render: (row) =>
      row.monitor_type === 'http_keyword' ? 'HTTP 关键字' : row.monitor_type.toUpperCase(),
  },
  {
    title: '状态条',
    key: 'recent_statuses',
    render: (row) =>
      h('div', { class: 'table-strip' }, [h(StatusStrip, { points: monitorPoints(row) })]),
  },
  {
    title: '最近日志',
    key: 'recent_checks',
    ellipsis: { tooltip: true },
    render: (row) => recentLog(row),
  },
  {
    title: '上次检查',
    key: 'last_checked_at',
    render: (row) =>
      row.last_checked_at ? new Date(row.last_checked_at).toLocaleString() : '等待首次检查',
  },
  {
    title: '操作',
    key: 'actions',
    render: (row) =>
      h(NSpace, null, {
        default: () => [
          button(PlayerPlay, '测试', 'success', () => test(row)),
          button(History, '详情', 'info', () => showHistory(row)),
          button(Bell, row.notify_enabled ? '通知已开' : '通知', 'warning', () =>
            openNotification(row),
          ),
        ],
      }),
  },
]

const checkColumns: DataTableColumns<MonitorCheck> = [
  { title: '时间', key: 'checked_at', render: (row) => new Date(row.checked_at).toLocaleString() },
  { title: '状态', key: 'status', render: (row) => h(StatusBadge, { status: row.status }) },
  {
    title: '延迟',
    key: 'latency_ms',
    render: (row) => (row.latency_ms == null ? '—' : `${row.latency_ms} ms`),
  },
  { title: '状态码', key: 'status_code', render: (row) => row.status_code ?? '—' },
  { title: '错误', key: 'error_message', ellipsis: { tooltip: true } },
  {
    title: '详情',
    key: 'extra_json',
    ellipsis: { tooltip: true },
    render: (row) => briefExtra(row),
  },
]
const checkPagination = reactive<PaginationProps>({
  pageSize: 20,
  pageSizes: [20, 50, 100],
  showSizePicker: true,
})
const tablePagination = reactive<PaginationProps>({
  pageSize: 20,
  pageSizes: [20, 50, 100],
  showSizePicker: true,
})

const drawerPoints = computed(() => [...checks.value].reverse().map(checkPoint))
const drawerStats = computed(() => {
  const total = checks.value.length
  const up = checks.value.filter((item) => item.status === 'up').length
  const latencies = checks.value
    .filter((item) => item.latency_ms != null)
    .map((item) => item.latency_ms ?? 0)
  return {
    uptime: total ? `${((up / total) * 100).toFixed(2)}%` : '等待数据',
    avgLatency: latencies.length
      ? `${Math.round(latencies.reduce((sum, item) => sum + item, 0) / latencies.length)} ms`
      : '—',
    total,
  }
})
const latencyPath = computed(() => {
  const values = [...checks.value]
    .reverse()
    .filter((item) => item.latency_ms != null)
    .slice(-80)
  if (values.length < 2) return ''
  const max = Math.max(...values.map((item) => item.latency_ms ?? 0), 1)
  return values
    .map((item, index) => {
      const x = (index / (values.length - 1)) * 100
      const y = 100 - ((item.latency_ms ?? 0) / max) * 86 - 7
      return `${x.toFixed(2)},${y.toFixed(2)}`
    })
    .join(' ')
})

function button(
  icon: typeof Bell,
  label: string,
  type: 'success' | 'info' | 'warning',
  onClick: () => void,
) {
  return h(
    NButton,
    { size: 'small', secondary: true, type, onClick },
    { icon: () => h(NIcon, { component: icon }), default: () => label },
  )
}

function searchableText(...values: Array<string | null | undefined>) {
  return values.filter(Boolean).join(' ').toLowerCase()
}

function spaceName(id?: string | null) {
  return spaces.value.find((item) => item.id === id)?.name || '默认空间'
}

function groupLabel(group: Group) {
  return `${spaceName(group.space_id)} / ${group.name}`
}

function serviceSpaceId(service: Service) {
  return (
    groups.value.find((group) => group.id === service.group_id)?.space_id || spaces.value[0]?.id
  )
}

function monitorPoints(monitor: Monitor): StatusPoint[] {
  const checks = [...monitor.recent_checks].reverse()
  if (checks.length) return checks.map(checkPoint)
  return monitor.recent_statuses.map((status) => ({ status }))
}

function checkPoint(check: MonitorCheck): StatusPoint {
  return {
    status: check.status,
    checked_at: check.checked_at,
    latency_ms: check.latency_ms,
    status_code: check.status_code,
    message: check.error_message || briefExtra(check),
  }
}

function recentLog(monitor: Monitor) {
  if (!monitor.recent_checks.length) return '等待首次检查'
  return monitor.recent_checks
    .slice(0, 3)
    .map(
      (item) =>
        `${new Date(item.checked_at).toLocaleTimeString()} ${item.status}${item.error_message ? ` · ${item.error_message}` : ''}`,
    )
    .join(' / ')
}

function serviceScope(monitor: Monitor) {
  const service = monitor.service_id ? serviceById.value.get(monitor.service_id) : null
  if (!service) return '未关联服务'
  const group = groupById.value.get(service.group_id)
  return `${service.name} / ${group ? groupLabel(group) : '默认空间 / 未分组'}`
}

function serviceCell(monitor: Monitor) {
  const service = monitor.service_id ? serviceById.value.get(monitor.service_id) : null
  if (!service) return h('span', { class: 'service-cell muted' }, '未关联服务')
  const group = groupById.value.get(service.group_id)
  return h('div', { class: 'service-cell' }, [
    h('strong', service.name),
    h('small', group ? groupLabel(group) : '默认空间 / 未分组'),
  ])
}

function monitorRowClass(row: Monitor) {
  const index = filteredMonitors.value.findIndex((item) => item.id === row.id)
  const current = row.service_id || row.id
  const previous = filteredMonitors.value[index - 1]?.service_id
  const next = filteredMonitors.value[index + 1]?.service_id
  return [
    previous === current ? 'same-service-cont' : '',
    next === current ? 'same-service-next' : '',
  ].join(' ')
}

function briefExtra(check: MonitorCheck) {
  if (!check.extra_json) return '—'
  try {
    const extra = JSON.parse(check.extra_json) as Record<string, unknown>
    if (typeof extra.health_status === 'string') return `Health: ${extra.health_status}`
    if (typeof extra.days_left === 'number') return `证书剩余 ${extra.days_left} 天`
    return JSON.stringify(extra)
  } catch {
    return check.extra_json
  }
}

function emptyMonitor(): MonitorInput {
  return {
    service_id: null,
    name: '',
    monitor_type: 'http',
    target_url: '',
    target_url_mode: 'custom',
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
    auth_type: 'none',
    auth_username: '',
    auth_password: '',
    domain: '',
    record_type: 'A',
    expected_value: '',
    cert_port: 443,
    cert_warning_days: 30,
    cert_critical_days: 0,
    enabled: true,
    notify_enabled: false,
    notification_channel_ids: [],
    notify_on_down: true,
    notify_on_recovery: true,
    notify_on_warning: true,
    notification_cooldown_sec: 300,
  }
}

async function load() {
  loading.value = true
  try {
    ;[monitors.value, services.value, groups.value, spaces.value, channels.value] =
      await Promise.all([
        monitorsApi.list(),
        servicesApi.list(),
        groupsApi.list(),
        groupsApi.spaces(),
        notificationsApi.channels(),
      ])
  } finally {
    loading.value = false
  }
}

async function test(monitor: Monitor) {
  message.loading('正在执行检查…', { duration: 0 })
  const result = await monitorsApi.test(monitor.id)
  message.destroyAll()
  if (result.status === 'up') message.success(`检查成功 · ${result.latency_ms ?? 0} ms`)
  else message.error(result.error_message || '检查失败')
  await load()
}

async function showHistory(monitor: Monitor) {
  selectedMonitor.value = monitor
  checks.value = await monitorsApi.checks(monitor.id)
  historyDrawer.value = true
}

function openNotification(monitor: Monitor) {
  notificationMonitor.value = monitor
  notificationForm.value = monitorToInput(monitor)
  notificationModal.value = true
}

async function saveNotification() {
  const monitor = notificationMonitor.value
  if (!monitor) return
  if (
    notificationForm.value.notify_enabled &&
    !notificationForm.value.notification_channel_ids.length
  ) {
    return message.warning('请选择至少一个通知通道')
  }
  notificationForm.value.notify_on_down = true
  notificationForm.value.notify_on_recovery = true
  notificationForm.value.notify_on_warning = true
  await monitorsApi.update(monitor.id, notificationForm.value)
  notificationModal.value = false
  message.success('监控通知设置已保存')
  await load()
}

onMounted(load)
</script>

<template>
  <header class="page-header">
    <div>
      <p>MONITOR LOGBOOK</p>
      <h1>监控日志</h1>
      <span>按分组和服务查看所有监控的状态条、最近日志与通知设置。</span>
    </div>
  </header>

  <div class="stat-grid">
    <NCard size="small"><NStatistic label="当前监控" :value="stats.total" /></NCard>
    <NCard size="small"><NStatistic label="正常" :value="stats.up" /></NCard>
    <NCard size="small"><NStatistic label="告警" :value="stats.warning" /></NCard>
    <NCard size="small"><NStatistic label="异常" :value="stats.down" /></NCard>
  </div>

  <NCard class="filter-card" size="small">
    <NSpace>
      <NSelect
        v-model:value="selectedSpaceId"
        :options="spaceOptions"
        class="filter-select"
        @update:value="selectedGroupId = ''; selectedServiceId = ''"
      />
      <NSelect
        v-model:value="selectedGroupId"
        :options="groupOptions"
        class="filter-select"
        @update:value="selectedServiceId = ''"
      />
      <NSelect v-model:value="selectedServiceId" :options="serviceOptions" class="filter-select" filterable />
      <NSelect v-model:value="selectedStatus" :options="statusOptions" class="filter-select" />
      <NInput
        v-model:value="search"
        clearable
        placeholder="搜索监控、服务、分组、地址或错误"
        class="filter-search"
      />
    </NSpace>
  </NCard>

  <NDataTable
    :columns="columns"
    :data="filteredMonitors"
    :loading="loading"
    :row-key="(row: Monitor) => row.id"
    :row-class-name="monitorRowClass"
    :pagination="tablePagination"
  />

  <NModal
    v-model:show="notificationModal"
    preset="card"
    title="监控通知设置"
    class="notify-monitor-modal"
    :mask-closable="false"
  >
    <NForm label-placement="top">
      <div class="switches">
        <label><NSwitch v-model:value="notificationForm.notify_enabled" /> 状态变化时通知</label>
      </div>
      <template v-if="notificationForm.notify_enabled">
        <NFormItem label="通知通道">
          <NSelect
            v-model:value="notificationForm.notification_channel_ids"
            :options="channelOptions"
            multiple
            filterable
            placeholder="选择通知通道"
          />
        </NFormItem>
      </template>
      <NButton type="primary" block @click="saveNotification">保存通知设置</NButton>
    </NForm>
  </NModal>

  <NDrawer
    v-model:show="historyDrawer"
    width="min(1280px, calc(100vw - 24px))"
  >
    <NDrawerContent
      :title="selectedMonitor ? `${selectedMonitor.name} · 检查日志` : '检查日志'"
      closable
      body-content-class="history-drawer-body"
    >
      <section v-if="selectedMonitor" class="monitor-detail">
        <div class="detail-head">
          <div>
            <small>{{ serviceScope(selectedMonitor) }}</small>
            <h2>{{ selectedMonitor.name }}</h2>
          </div>
          <StatusBadge :status="selectedMonitor.current_status" />
        </div>

        <NCard size="small" class="status-card">
          <div class="status-card-head">
            <span>最近检查状态</span>
            <NTag size="small" :bordered="false">{{ drawerStats.total }} 次记录</NTag>
          </div>
          <StatusStrip :points="drawerPoints" title="最近检查状态" />
        </NCard>

        <div class="detail-stats">
          <NCard size="small"><NStatistic label="在线时间（当前样本）" :value="drawerStats.uptime" /></NCard>
          <NCard size="small"><NStatistic label="平均响应" :value="drawerStats.avgLatency" /></NCard>
          <NCard size="small"><NStatistic label="检查间隔" :value="`${selectedMonitor.interval_sec} 秒`" /></NCard>
        </div>

        <NCard size="small" class="latency-card">
          <div class="status-card-head">
            <span>平均 Ping 延迟</span>
            <small>最近 {{ checks.filter((item) => item.latency_ms != null).length }} 个有效样本</small>
          </div>
          <svg v-if="latencyPath" viewBox="0 0 100 100" preserveAspectRatio="none" class="latency-chart">
            <polyline :points="`0,100 ${latencyPath} 100,100`" class="latency-fill" />
            <polyline :points="latencyPath" class="latency-line" />
          </svg>
          <div v-else class="empty-chart">暂无延迟数据</div>
        </NCard>

        <NDescriptions :column="2" size="small" label-placement="left" class="monitor-meta">
          <NDescriptionsItem label="类型">{{ selectedMonitor.monitor_type.toUpperCase() }}</NDescriptionsItem>
          <NDescriptionsItem label="目标">{{ selectedMonitor.target_url || selectedMonitor.domain || '随服务地址' }}</NDescriptionsItem>
          <NDescriptionsItem label="上次检查">{{ selectedMonitor.last_checked_at ? new Date(selectedMonitor.last_checked_at).toLocaleString() : '等待首次检查' }}</NDescriptionsItem>
          <NDescriptionsItem label="最近错误">{{ selectedMonitor.last_error || '—' }}</NDescriptionsItem>
        </NDescriptions>
      </section>
      <NDataTable
        class="check-table"
        :columns="checkColumns"
        :data="checks"
        size="small"
        :pagination="checkPagination"
      />
      <template #footer>
        <NButton @click="historyDrawer = false">关闭</NButton>
      </template>
    </NDrawerContent>
  </NDrawer>
</template>

<style scoped>
.page-header {
  display: flex;
  align-items: end;
  justify-content: space-between;
  gap: 2rem;
  margin-bottom: 1.4rem;
}
.page-header p {
  margin: 0;
  color: var(--sc-success);
  font-family: "IBM Plex Mono", monospace;
  font-size: 0.68rem;
  letter-spacing: 0.2em;
}
.page-header h1 {
  margin: 0.35rem 0;
  font-size: 2.35rem;
}
.page-header span {
  color: var(--sc-muted);
}
.stat-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr));
  gap: 0.75rem;
  margin-bottom: 0.75rem;
}
.filter-card {
  margin-bottom: 0.75rem;
}
.filter-select {
  width: 14rem;
}
.filter-search {
  width: min(34rem, 100%);
}
:deep(.table-strip) { width: min(13rem, 100%); min-width: 0; }
:global(.history-drawer-body) { min-width: 0; overflow-x: hidden; }
:global(.history-drawer-body > *) { min-width: 0; }
:deep(.same-service-cont td) { border-top-color: transparent !important; }
:deep(.same-service-next td:first-child) { border-bottom-left-radius: 0; }
:deep(.service-cell) { display: grid; gap: 0.12rem; line-height: 1.2; }
:deep(.service-cell strong) { font-size: 0.82rem; }
:deep(.service-cell small) { color: var(--sc-muted); font-size: 0.68rem; }
:deep(.service-cell.muted) { color: var(--sc-muted); }
.monitor-detail { display: grid; min-width: 0; gap: 0.85rem; margin-bottom: 1rem; overflow: hidden; }
.detail-head { display: flex; align-items: flex-end; justify-content: space-between; gap: 1rem; }
.detail-head > div { min-width: 0; }
.detail-head small { color: var(--sc-muted); }
.detail-head h2 { margin: 0.2rem 0 0; overflow-wrap: anywhere; font-size: 1.8rem; }
.status-card, .latency-card { background: var(--sc-card); }
:deep(.status-card .n-card__content), :deep(.latency-card .n-card__content) { min-width: 0; overflow: hidden; }
.status-card-head { display: flex; align-items: center; justify-content: space-between; gap: 1rem; margin-bottom: 0.7rem; color: var(--sc-muted); font-size: 0.78rem; }
.detail-stats { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 0.75rem; }
.latency-chart { width: 100%; height: 12rem; border-bottom: 1px solid rgb(148 163 184 / 18%); }
.latency-line { fill: none; stroke: #4ade80; stroke-width: 2.2; vector-effect: non-scaling-stroke; }
.latency-fill { fill: rgb(74 222 128 / 16%); stroke: none; }
.empty-chart { display: grid; height: 12rem; place-items: center; color: var(--sc-muted); }
.monitor-meta { padding: 0.8rem; border: 1px solid var(--sc-border); border-radius: 0.75rem; }
.check-table { min-width: 0; }
.notify-monitor-modal {
  width: min(36rem, calc(100vw - 2rem));
}
.switches {
  display: flex;
  gap: 1rem;
  margin-bottom: 1rem;
}
.switches label {
  display: flex;
  align-items: center;
  gap: 0.4rem;
}
.form-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 0 1rem;
}
@media (max-width: 760px) {
  .page-header {
    align-items: flex-start;
    flex-direction: column;
  }
  .filter-select {
    width: min(100%, 18rem);
  }
  .filter-search {
    width: min(100%, 18rem);
  }
  .form-grid {
    grid-template-columns: 1fr;
  }
}
</style>
