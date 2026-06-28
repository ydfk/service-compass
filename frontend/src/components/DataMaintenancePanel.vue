<script setup lang="ts">
import { Archive, CloudUpload, DatabaseExport, Photo } from '@vicons/tabler'
import {
  NAlert,
  NButton,
  NCard,
  NDataTable,
  NForm,
  NFormItem,
  NIcon,
  NInput,
  NInputNumber,
  NSelect,
  NSwitch,
  useMessage,
  type DataTableColumns,
  type PaginationProps,
} from 'naive-ui'
import { onMounted, reactive, ref, shallowRef } from 'vue'
import { maintenanceApi, type BackupConfig } from '../api/maintenance'
import type { BackupRun } from '../types'

const backup = reactive<BackupConfig>({
  enabled: false,
  schedule_time: '03:00',
  target_type: 'local',
  local_dir: '',
  webdav_url: '',
  webdav_username: '',
  webdav_password: '',
  aliyun_oss_endpoint: '',
  aliyun_oss_region: '',
  aliyun_oss_bucket: '',
  aliyun_oss_prefix: '',
  aliyun_oss_access_key_id: '',
  aliyun_oss_access_key_secret: '',
  retention_count: 7,
})
const runs = ref<BackupRun[]>([])
const loading = shallowRef(false)
const fileInput = ref<HTMLInputElement | null>(null)
const message = useMessage()
const pagination = reactive<PaginationProps>({
  page: 1,
  pageSize: 20,
  itemCount: 0,
  onChange: (page: number) => {
    pagination.page = page
    void loadRuns()
  },
})
const runColumns: DataTableColumns<BackupRun> = [
  { title: '类型', key: 'run_type', render: (row) => runType(row.run_type) },
  { title: '目标', key: 'target_type', render: (row) => targetType(row.target_type) },
  { title: '状态', key: 'status', render: (row) => statusText(row.status) },
  {
    title: '大小',
    key: 'file_size',
    render: (row) => formatSize(row.file_size),
  },
  {
    title: '文件 / 地址',
    key: 'filename',
    ellipsis: { tooltip: true },
    render: (row) => row.filename || '—',
  },
  {
    title: '时间（本地）',
    key: 'started_at',
    render: (row) => new Date(row.started_at).toLocaleString(),
  },
  {
    title: '耗时',
    key: 'duration',
    render: (row) => formatDuration(row.started_at, row.finished_at),
  },
  {
    title: '结果',
    key: 'result',
    ellipsis: { tooltip: true },
    render: resultSummary,
  },
]

async function load() {
  const config = await maintenanceApi.backupConfig()
  Object.assign(backup, {
    ...config,
    webdav_password: '',
    aliyun_oss_access_key_secret: '',
  })
  await loadRuns()
}

async function loadRuns() {
  const page = await maintenanceApi.backupRuns(pagination.page || 1, pagination.pageSize || 20)
  runs.value = page.items
  pagination.itemCount = page.total
}

async function saveBackupConfig() {
  loading.value = true
  try {
    const config = await maintenanceApi.updateBackupConfig(backup)
    Object.assign(backup, {
      ...config,
      webdav_password: '',
      aliyun_oss_access_key_secret: '',
    })
    message.success('备份计划已保存')
  } finally {
    loading.value = false
  }
}

async function runBackup() {
  loading.value = true
  try {
    const result = await maintenanceApi.runBackup()
    message.success(`备份已完成：${result.path}`)
  } catch (error) {
    message.error(`备份失败：${errorMessage(error)}`)
  } finally {
    await refreshRunsSafely()
    loading.value = false
  }
}

async function testBackupTarget() {
  loading.value = true
  try {
    const result = await maintenanceApi.testBackupTarget(backup)
    message.success(`备份目标可用：${result.path}`)
  } catch (error) {
    message.error(`备份目标测试失败：${errorMessage(error)}`)
  } finally {
    await refreshRunsSafely()
    loading.value = false
  }
}

async function exportConfig() {
  loading.value = true
  try {
    await maintenanceApi.exportConfig()
  } finally {
    loading.value = false
  }
}

async function localizeIcons() {
  loading.value = true
  try {
    const result = await maintenanceApi.localizeIcons()
    const failed = result.failed.length ? `，失败 ${result.failed.length} 个` : ''
    message.success(`已本地化 ${result.success} 个图标${failed}`)
  } finally {
    loading.value = false
  }
}

async function importConfig(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  loading.value = true
  try {
    const result = await maintenanceApi.importConfig(file)
    message.warning(result.message)
  } finally {
    loading.value = false
    input.value = ''
  }
}

function targetType(value: string) {
  const labels: Record<string, string> = {
    local: '本地目录',
    webdav: 'WebDAV',
    aliyun_oss: '阿里云 OSS',
  }
  return labels[value] || value
}

function runType(value: string) {
  const labels: Record<string, string> = {
    manual: '手动',
    scheduled: '计划',
    test: '测试',
  }
  return labels[value] || value
}

function statusText(value: string) {
  const labels: Record<string, string> = {
    running: '运行中',
    success: '成功',
    failed: '失败',
  }
  return labels[value] || value
}

function resultSummary(row: BackupRun) {
  return (
    [
      row.http_status_code ? `HTTP ${row.http_status_code}` : '',
      row.error_message || row.response_summary || '',
    ]
      .filter(Boolean)
      .join(' · ') || '—'
  )
}

function formatSize(value?: number | null) {
  if (!value) return '—'
  if (value < 1024) return `${value} B`
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`
  return `${(value / 1024 / 1024).toFixed(1)} MB`
}

function formatDuration(startedAt: string, finishedAt?: string | null) {
  if (!finishedAt) return '—'
  const duration = new Date(finishedAt).getTime() - new Date(startedAt).getTime()
  if (!Number.isFinite(duration) || duration < 0) return '—'
  if (duration < 1000) return `${duration} ms`
  return `${(duration / 1000).toFixed(1)} s`
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '未知错误'
}

async function refreshRunsSafely() {
  try {
    await loadRuns()
  } catch {
    message.warning('备份日志刷新失败，请稍后手动刷新页面')
  }
}

onMounted(load)
</script>

<template>
  <div class="maintenance-layout">
    <NCard title="数据维护" class="maintenance-card maintenance-main">
      <div class="action-row">
        <NButton :loading="loading" @click="exportConfig">
          <template #icon><NIcon :component="DatabaseExport" /></template>导出配置包
        </NButton>
        <NButton :loading="loading" @click="fileInput?.click()">
          <template #icon><NIcon :component="CloudUpload" /></template>导入配置包
        </NButton>
        <NButton :loading="loading" @click="localizeIcons">
          <template #icon><NIcon :component="Photo" /></template>图标本地化
        </NButton>
        <input ref="fileInput" type="file" accept=".zip,application/zip" class="file-input" @change="importConfig" />
      </div>
      <NAlert type="warning" :bordered="false" class="restore-note">
        导入会整体替换数据库、密钥和图标目录，导入后需要重启容器生效。
      </NAlert>

      <NForm label-placement="top" class="backup-form">
        <div class="form-grid">
          <NFormItem label="启用每日备份">
            <NSwitch v-model:value="backup.enabled" />
          </NFormItem>
          <NFormItem label="每日执行时间">
            <NInput v-model:value="backup.schedule_time" placeholder="03:00" />
          </NFormItem>
          <NFormItem label="备份目标">
            <NSelect
              v-model:value="backup.target_type"
              :options="[
                { label: '本地目录', value: 'local' },
                { label: 'WebDAV', value: 'webdav' },
                { label: '阿里云 OSS', value: 'aliyun_oss' },
              ]"
            />
          </NFormItem>
          <NFormItem label="保留份数">
            <NInputNumber v-model:value="backup.retention_count" :min="1" :max="365" />
          </NFormItem>
          <NFormItem v-if="backup.target_type === 'local'" label="本地备份目录" class="span-2">
            <NInput v-model:value="backup.local_dir" placeholder="/data/backups 或宿主机挂载目录" />
          </NFormItem>
          <template v-else-if="backup.target_type === 'webdav'">
            <NFormItem label="WebDAV 地址" class="span-2">
              <NInput v-model:value="backup.webdav_url" placeholder="目录地址，例如 https://dav.example.com/dav/service-compass" />
            </NFormItem>
            <NFormItem label="WebDAV 用户名">
              <NInput v-model:value="backup.webdav_username" placeholder="用户名" />
            </NFormItem>
            <NFormItem label="WebDAV 密码">
              <NInput
                v-model:value="backup.webdav_password"
                type="password"
                show-password-on="click"
                :placeholder="backup.has_webdav_password ? '留空则保留现有密码' : '密码'"
              />
            </NFormItem>
          </template>
          <template v-else>
            <NFormItem label="OSS Endpoint">
              <NInput v-model:value="backup.aliyun_oss_endpoint" placeholder="https://s3.oss-cn-hangzhou.aliyuncs.com" />
            </NFormItem>
            <NFormItem label="Region">
              <NInput v-model:value="backup.aliyun_oss_region" placeholder="oss-cn-hangzhou" />
            </NFormItem>
            <NFormItem label="Bucket">
              <NInput v-model:value="backup.aliyun_oss_bucket" placeholder="service-compass-backup" />
            </NFormItem>
            <NFormItem label="对象前缀">
              <NInput v-model:value="backup.aliyun_oss_prefix" placeholder="service-compass/" />
            </NFormItem>
            <NFormItem label="AccessKey ID">
              <NInput v-model:value="backup.aliyun_oss_access_key_id" placeholder="AccessKey ID" />
            </NFormItem>
            <NFormItem label="AccessKey Secret">
              <NInput
                v-model:value="backup.aliyun_oss_access_key_secret"
                type="password"
                show-password-on="click"
                :placeholder="backup.has_aliyun_oss_access_key_secret ? '留空则保留现有 Secret' : 'AccessKey Secret'"
              />
            </NFormItem>
          </template>
        </div>
        <div class="backup-actions">
          <NButton type="primary" :loading="loading" @click="saveBackupConfig">保存备份计划</NButton>
          <NButton type="info" secondary :loading="loading" @click="testBackupTarget">测试目标</NButton>
          <NButton type="success" secondary :loading="loading" @click="runBackup">
            <template #icon><NIcon :component="Archive" /></template>立即备份
          </NButton>
        </div>
      </NForm>
    </NCard>

    <NCard title="备份日志" class="maintenance-card maintenance-logs">
      <NDataTable
        :columns="runColumns"
        :data="runs"
        :pagination="pagination"
        size="small"
        :scroll-x="980"
        :row-key="(row: BackupRun) => row.id"
      />
    </NCard>
  </div>
</template>

<style scoped>
.maintenance-layout {
  display: grid;
  grid-template-columns: minmax(26rem, 0.88fr) minmax(32rem, 1.12fr);
  gap: 1rem;
  align-items: start;
  margin-top: 1rem;
}
.maintenance-card {
  min-width: 0;
}
.maintenance-logs :deep(.n-data-table) {
  --n-td-padding: 8px 10px;
  --n-th-padding: 8px 10px;
}
.action-row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.6rem;
}
.file-input {
  display: none;
}
.restore-note {
  margin-top: 0.8rem;
}
.backup-form {
  margin-top: 1rem;
}
.form-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0 1rem;
}
.span-2 {
  grid-column: span 2;
}
.backup-actions {
  display: flex;
  gap: 0.6rem;
}
@media (max-width: 1120px) {
  .maintenance-layout {
    grid-template-columns: 1fr;
  }
}
@media (max-width: 680px) {
  .form-grid {
    grid-template-columns: 1fr;
  }
  .span-2 {
    grid-column: auto;
  }
}
</style>
