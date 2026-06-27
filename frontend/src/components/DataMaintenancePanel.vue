<script setup lang="ts">
import { Archive, CloudUpload, DatabaseExport, Photo } from '@vicons/tabler'
import {
  NAlert,
  NButton,
  NCard,
  NForm,
  NFormItem,
  NIcon,
  NInput,
  NInputNumber,
  NSelect,
  NSwitch,
  useMessage,
} from 'naive-ui'
import { onMounted, reactive, ref } from 'vue'
import { maintenanceApi, type BackupConfig } from '../api/maintenance'

const backup = reactive<BackupConfig>({
  enabled: false,
  schedule_time: '03:00',
  target_type: 'local',
  local_dir: '',
  webdav_url: '',
  webdav_username: '',
  webdav_password: '',
  retention_count: 7,
})
const loading = ref(false)
const fileInput = ref<HTMLInputElement | null>(null)
const message = useMessage()

async function load() {
  const config = await maintenanceApi.backupConfig()
  Object.assign(backup, { ...config, webdav_password: '' })
}

async function saveBackupConfig() {
  loading.value = true
  try {
    const config = await maintenanceApi.updateBackupConfig(backup)
    Object.assign(backup, { ...config, webdav_password: '' })
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
    await load()
  } finally {
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

onMounted(load)
</script>

<template>
  <NCard title="数据维护" class="maintenance-card">
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
            ]"
          />
        </NFormItem>
        <NFormItem label="保留份数">
          <NInputNumber v-model:value="backup.retention_count" :min="1" :max="365" />
        </NFormItem>
        <NFormItem v-if="backup.target_type === 'local'" label="本地备份目录" class="span-2">
          <NInput v-model:value="backup.local_dir" placeholder="/data/backups 或宿主机挂载目录" />
        </NFormItem>
        <template v-else>
          <NFormItem label="WebDAV 地址" class="span-2">
            <NInput v-model:value="backup.webdav_url" placeholder="https://dav.example.com/service-compass" />
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
      </div>
      <div class="backup-actions">
        <NButton type="primary" :loading="loading" @click="saveBackupConfig">保存备份计划</NButton>
        <NButton type="success" secondary :loading="loading" @click="runBackup">
          <template #icon><NIcon :component="Archive" /></template>立即备份
        </NButton>
      </div>
    </NForm>
  </NCard>
</template>

<style scoped>
.maintenance-card {
  max-width: 58rem;
  margin-top: 1rem;
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
@media (max-width: 680px) {
  .form-grid {
    grid-template-columns: 1fr;
  }
  .span-2 {
    grid-column: auto;
  }
}
</style>
