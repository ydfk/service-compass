<script setup lang="ts">
import { Activity, Edit, History, PlayerPlay, Plus, Trash } from '@vicons/tabler'
import {
  NButton,
  NCard,
  NDataTable,
  NDrawer,
  NDrawerContent,
  NIcon,
  NModal,
  NSelect,
  NSpace,
  NStatistic,
  useDialog,
  useMessage,
  type DataTableColumns,
} from 'naive-ui'
import { computed, h, onMounted, ref } from 'vue'
import { groupsApi } from '../api/groups'
import { monitorsApi } from '../api/monitors'
import { servicesApi } from '../api/services'
import MonitorForm from '../components/MonitorForm.vue'
import StatusBadge from '../components/StatusBadge.vue'
import type { Group, Monitor, MonitorCheck, MonitorInput, Service } from '../types'
import { UNGROUPED_ID } from '../utils/serviceForms'

const monitors = ref<Monitor[]>([])
const services = ref<Service[]>([])
const groups = ref<Group[]>([])
const checks = ref<MonitorCheck[]>([])
const editing = ref<Monitor | null>(null)
const selectedMonitor = ref<Monitor | null>(null)
const form = ref<MonitorInput>(emptyMonitor())
const modal = ref(false)
const historyDrawer = ref(false)
const loading = ref(false)
const selectedGroupId = ref('')
const selectedServiceId = ref('')
const message = useMessage()
const dialog = useDialog()

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
    title: '目标',
    key: 'target_url',
    ellipsis: { tooltip: true },
    render: (row) => monitorTarget(row),
  },
  {
    title: '延迟',
    key: 'last_latency_ms',
    render: (row) => (row.last_latency_ms == null ? '—' : `${row.last_latency_ms} ms`),
  },
  { title: '证书', key: 'cert', render: (row) => certificateDays(row) },
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
          button(Edit, '编辑', () => open(row)),
          button(Trash, '删除', () => remove(row)),
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

function button(icon: typeof Edit, label: string, onClick: () => void) {
  return h(
    NButton,
    { size: 'small', quaternary: true, onClick },
    { icon: () => h(NIcon, { component: icon }), default: () => label },
  )
}

function serviceScope(monitor: Monitor) {
  const service = monitor.service_id ? serviceById.value.get(monitor.service_id) : null
  if (!service) return '未关联服务'
  const group = groupById.value.get(service.group_id)
  return `${service.name} / ${group?.name || '未分组'}`
}

function monitorTarget(monitor: Monitor) {
  if (monitor.monitor_type === 'docker') return serviceScope(monitor)
  return monitor.domain || monitor.target_url || `${monitor.target_url_mode} 地址`
}

function certificateDays(monitor: Monitor) {
  if (monitor.monitor_type !== 'cert' || !monitor.last_extra_json) return '—'
  try {
    const extra = JSON.parse(monitor.last_extra_json) as {
      days_left?: number
      warning_days?: number
    }
    return extra.days_left == null
      ? '—'
      : `${extra.days_left} 天 / 提前 ${extra.warning_days ?? 30} 天提醒`
  } catch {
    return '—'
  }
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
  }
}

async function load() {
  loading.value = true
  try {
    ;[monitors.value, services.value, groups.value] = await Promise.all([
      monitorsApi.list(),
      servicesApi.list(),
      groupsApi.list(),
    ])
  } finally {
    loading.value = false
  }
}

function open(monitor?: Monitor) {
  editing.value = monitor ?? null
  form.value = monitor
    ? {
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
      }
    : emptyMonitor()
  modal.value = true
}

async function save() {
  const http = ['http', 'http_keyword'].includes(form.value.monitor_type)
  if (
    !form.value.name ||
    (http && form.value.target_url_mode === 'custom' && !form.value.target_url)
  )
    return message.warning('请填写监控名称与目标 URL')
  if (['dns', 'cert'].includes(form.value.monitor_type) && !form.value.domain)
    return message.warning('请填写域名')
  if (editing.value) await monitorsApi.update(editing.value.id, form.value)
  else await monitorsApi.create(form.value)
  modal.value = false
  message.success('监控已保存，调度器将自动执行')
  await load()
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

function remove(monitor: Monitor) {
  dialog.warning({
    title: '删除监控',
    content: `确认删除 ${monitor.name} 及其历史记录？`,
    positiveText: '删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      await monitorsApi.remove(monitor.id)
      await load()
    },
  })
}

onMounted(load)
</script>

<template>
  <header class="page-header">
    <div>
      <p>MONITOR BEARINGS</p>
      <h1>监控</h1>
      <span>按分组和服务查看 HTTP、Docker、DNS 与证书检查日志。</span>
    </div>
    <NButton type="primary" @click="open()">
      <template #icon><NIcon :component="Plus" /></template>
      新建监控
    </NButton>
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

  <NModal v-model:show="modal" preset="card" :title="editing ? '编辑监控' : '新建监控'" class="monitor-modal">
    <MonitorForm v-model="form" :services="services" />
    <NButton type="primary" block @click="save">
      <template #icon><NIcon :component="Activity" /></template>
      保存监控
    </NButton>
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
.monitor-modal {
  width: min(52rem, calc(100vw - 2rem));
}
@media (max-width: 760px) {
  .page-header {
    align-items: flex-start;
    flex-direction: column;
  }
  .filter-select {
    width: min(100%, 18rem);
  }
}
</style>
