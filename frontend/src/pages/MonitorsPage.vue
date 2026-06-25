<script setup lang="ts">
import { Bell, History, PlayerPlay } from '@vicons/tabler'
import {
  NButton,
  NCard,
  NDataTable,
  NDrawer,
  NDrawerContent,
  NForm,
  NFormItem,
  NIcon,
  NInputNumber,
  NModal,
  NSelect,
  NSpace,
  NStatistic,
  NSwitch,
  useMessage,
  type DataTableColumns,
} from 'naive-ui'
import { computed, h, onMounted, ref } from 'vue'
import { groupsApi } from '../api/groups'
import { monitorsApi } from '../api/monitors'
import { notificationsApi } from '../api/notifications'
import { servicesApi } from '../api/services'
import StatusBadge from '../components/StatusBadge.vue'
import type {
  Group,
  Monitor,
  MonitorCheck,
  MonitorInput,
  NotificationChannel,
  Service,
  Status,
} from '../types'
import { monitorToInput, UNGROUPED_ID } from '../utils/serviceForms'

const monitors = ref<Monitor[]>([])
const services = ref<Service[]>([])
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
const message = useMessage()

const serviceById = computed(() => new Map(services.value.map((item) => [item.id, item])))
const groupById = computed(() => new Map(groups.value.map((item) => [item.id, item])))
const groupOptions = computed(() => [
  { label: '全部分组', value: '' },
  { label: '未分组', value: UNGROUPED_ID },
  ...groups.value.map((item) => ({ label: item.name, value: item.id })),
])
const serviceOptions = computed(() => {
  const filtered = selectedGroupId.value
    ? services.value.filter((item) => item.group_id === selectedGroupId.value)
    : services.value
  return [
    { label: '全部服务', value: '' },
    ...filtered.map((item) => ({ label: item.name, value: item.id })),
  ]
})
const channelOptions = computed(() =>
  channels.value.map((item) => ({ label: item.name, value: item.id })),
)
const filteredMonitors = computed(() =>
  monitors.value.filter((monitor) => {
    if (selectedServiceId.value) return monitor.service_id === selectedServiceId.value
    if (!selectedGroupId.value) return true
    const service = monitor.service_id ? serviceById.value.get(monitor.service_id) : null
    return service?.group_id === selectedGroupId.value
  }),
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
    render: (row) => serviceScope(row),
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
    render: (row) => hStatusStrip(row.recent_statuses),
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
          button(PlayerPlay, '测试', () => test(row)),
          button(History, '日志', () => showHistory(row)),
          button(Bell, row.notify_enabled ? '通知已开' : '通知', () => openNotification(row)),
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

function button(icon: typeof Bell, label: string, onClick: () => void) {
  return h(
    NButton,
    { size: 'small', quaternary: true, onClick },
    { icon: () => h(NIcon, { component: icon }), default: () => label },
  )
}

function hStatusStrip(statuses: Status[]) {
  const values = statuses.length ? statuses : ['unknown']
  return h(
    'div',
    { class: 'monitor-status-strip', title: '最近 30 次检查' },
    values.map((status, index) => h('i', { key: index, class: status })),
  )
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
  return `${service.name} / ${group?.name || '未分组'}`
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
    ;[monitors.value, services.value, groups.value, channels.value] = await Promise.all([
      monitorsApi.list(),
      servicesApi.list(),
      groupsApi.list(),
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
        v-model:value="selectedGroupId"
        :options="groupOptions"
        class="filter-select"
        @update:value="selectedServiceId = ''"
      />
      <NSelect v-model:value="selectedServiceId" :options="serviceOptions" class="filter-select" filterable />
    </NSpace>
  </NCard>

  <NDataTable
    :columns="columns"
    :data="filteredMonitors"
    :loading="loading"
    :row-key="(row: Monitor) => row.id"
  />

  <NModal v-model:show="notificationModal" preset="card" title="监控通知设置" class="notify-monitor-modal">
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
        <div class="form-grid">
          <NFormItem label="离线通知"><NSwitch v-model:value="notificationForm.notify_on_down" /></NFormItem>
          <NFormItem label="恢复通知"><NSwitch v-model:value="notificationForm.notify_on_recovery" /></NFormItem>
          <NFormItem label="警告通知"><NSwitch v-model:value="notificationForm.notify_on_warning" /></NFormItem>
          <NFormItem label="冷却时间（秒）">
            <NInputNumber v-model:value="notificationForm.notification_cooldown_sec" :min="0" />
          </NFormItem>
        </div>
      </template>
      <NButton type="primary" block @click="saveNotification">保存通知设置</NButton>
    </NForm>
  </NModal>

  <NDrawer v-model:show="historyDrawer" :width="820">
    <NDrawerContent :title="selectedMonitor ? `${selectedMonitor.name} · 检查日志` : '检查日志'">
      <NDataTable :columns="checkColumns" :data="checks" size="small" />
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
:deep(.monitor-status-strip) {
  display: flex;
  width: min(12rem, 100%);
  height: 0.55rem;
  gap: 2px;
}
:deep(.monitor-status-strip i) {
  min-width: 2px;
  flex: 1;
  border-radius: 1px;
  background: #334155;
}
:deep(.monitor-status-strip i.up) {
  background: #34d399;
}
:deep(.monitor-status-strip i.down) {
  background: #fb7185;
}
:deep(.monitor-status-strip i.warning) {
  background: #fbbf24;
}
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
  .form-grid {
    grid-template-columns: 1fr;
  }
}
</style>
