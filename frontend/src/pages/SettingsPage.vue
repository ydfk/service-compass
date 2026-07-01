<script setup lang="ts">
import { DeviceFloppy, Key } from '@vicons/tabler'
import {
  NAlert,
  NButton,
  NCard,
  NForm,
  NFormItem,
  NIcon,
  NInput,
  NInputNumber,
  useMessage,
} from 'naive-ui'
import { onMounted, reactive, ref } from 'vue'
import { api } from '../api/client'
import { settingsApi } from '../api/settings'
import { useAuthStore } from '../stores/auth'

const retentionDays = ref(30)
const logRetentionDays = ref(30)
const certExpiryWarningDays = ref(30)
const notificationCooldownSec = ref(300)
const dashboardRefreshIntervalSec = ref(30)
const anonymousAccessCidrs = ref('')
const credentials = reactive({ current_password: '', username: '', new_password: '' })
const message = useMessage()
const auth = useAuthStore()

async function save() {
  await settingsApi.update({
    retention_days: retentionDays.value,
    log_retention_days: logRetentionDays.value,
    cert_expiry_warning_days: certExpiryWarningDays.value,
    notification_cooldown_sec: notificationCooldownSec.value,
    dashboard_refresh_interval_sec: dashboardRefreshIntervalSec.value,
    anonymous_access_cidrs: anonymousAccessCidrs.value,
  })
  message.success('设置已保存')
}

async function updateCredentials() {
  await api('/api/auth/credentials', { method: 'PUT', body: JSON.stringify(credentials) })
  message.success('账号已修改，请重新登录')
  auth.clear()
  window.location.assign('/login')
}

onMounted(async () => {
  const settings = await settingsApi.get()
  retentionDays.value = settings.retention_days
  logRetentionDays.value = settings.log_retention_days
  certExpiryWarningDays.value = settings.cert_expiry_warning_days
  notificationCooldownSec.value = settings.notification_cooldown_sec
  dashboardRefreshIntervalSec.value = settings.dashboard_refresh_interval_sec
  anonymousAccessCidrs.value = settings.anonymous_access_cidrs
  credentials.username = auth.username || (await api<{ username: string }>('/api/auth/me')).username
})
</script>

<template>
  <header class="page-header"><p>SYSTEM SETTINGS</p><h1>设置</h1><span>管理实例参数与管理员账号。</span></header>
  <div class="settings-grid">
    <NCard title="常规">
      <NFormItem label="检查记录保留天数"><NInputNumber v-model:value="retentionDays" :min="1" :max="365" /></NFormItem>
      <NFormItem label="系统日志保留天数"><NInputNumber v-model:value="logRetentionDays" :min="1" :max="365" /></NFormItem>
      <NFormItem label="证书到期提醒提前天数"><NInputNumber v-model:value="certExpiryWarningDays" :min="1" :max="365" /></NFormItem>
      <NFormItem label="通知冷却时间（秒）"><NInputNumber v-model:value="notificationCooldownSec" :min="0" :max="86400" /></NFormItem>
      <NFormItem label="首页自动刷新间隔（秒）"><NInputNumber v-model:value="dashboardRefreshIntervalSec" :min="5" :max="3600" /></NFormItem>
      <NFormItem label="允许匿名访问的内网网段">
        <NInput
          v-model:value="anonymousAccessCidrs"
          type="textarea"
          :autosize="{ minRows: 6, maxRows: 10 }"
          placeholder="每行一个网段，例如 192.168.1.0/24。留空表示所有来源都必须登录。"
        />
      </NFormItem>
      <NButton type="primary" @click="save"><template #icon><NIcon :component="DeviceFloppy" /></template>保存设置</NButton>
    </NCard>
    <NCard title="管理员账号">
      <NForm label-placement="top">
        <NFormItem label="用户名"><NInput v-model:value="credentials.username" autocomplete="username" /></NFormItem>
        <NFormItem label="当前密码"><NInput v-model:value="credentials.current_password" type="password" show-password-on="click" autocomplete="current-password" /></NFormItem>
        <NFormItem label="新密码（至少 6 位）"><NInput v-model:value="credentials.new_password" type="password" show-password-on="click" autocomplete="new-password" /></NFormItem>
        <NButton type="primary" :disabled="!credentials.current_password || credentials.new_password.length < 6" @click="updateCredentials"><template #icon><NIcon :component="Key" /></template>修改并重新登录</NButton>
      </NForm>
    </NCard>
  </div>
  <NAlert type="info" :bordered="false" class="note">非上述网段访问时，首页数据和管理内容都必须登录后查看。通知凭据和加密密钥不会以明文返回前端。</NAlert>
</template>

<style scoped>
.page-header p { margin: 0; color: #5da9ff; font-family: "IBM Plex Mono", monospace; font-size: 0.68rem; letter-spacing: 0.2em; }
.page-header h1 { margin: 0.35rem 0; font-size: 2.1rem; }
.page-header span { color: #75859b; }
.settings-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(18rem, 1fr)); gap: 1rem; max-width: 58rem; margin-top: 1.5rem; }
.note { max-width: 58rem; margin-top: 1rem; }
</style>
