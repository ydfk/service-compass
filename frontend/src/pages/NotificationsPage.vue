<script setup lang="ts">
import { Bell, Check, Edit, PlayerPlay, Trash } from '@vicons/tabler'
import {
  NButton,
  NCard,
  NForm,
  NFormItem,
  NIcon,
  NInput,
  NModal,
  NSelect,
  NSpace,
  NSwitch,
  NTag,
  useDialog,
  useMessage,
} from 'naive-ui'
import { computed, onMounted, ref } from 'vue'
import { notificationsApi, type NotificationChannelInput } from '../api/notifications'
import { servicesApi } from '../api/services'
import type { NotificationChannel, Service } from '../types'

interface ChannelSecrets {
  server_url: string
  device_key: string
  sound: string
  group: string
  url: string
  method: string
  headers: string
  mode: string
  base_url: string
  token: string
  user_ids: string
  verify_tls: boolean
  service_scope: 'all' | 'selected'
  service_ids: string[]
}

const channels = ref<NotificationChannel[]>([])
const services = ref<Service[]>([])
const editingChannel = ref<NotificationChannel | null>(null)
const channelModal = ref(false)
const channelForm = ref<NotificationChannelInput>(emptyChannel())
const secrets = ref<ChannelSecrets>(emptySecrets())
const message = useMessage()
const dialog = useDialog()

const serviceOptions = computed(() =>
  services.value.map((item) => ({ label: item.name, value: item.id })),
)

function emptyChannel(): NotificationChannelInput {
  return { name: '', channel_type: 'bark', enabled: true }
}

function emptySecrets(): ChannelSecrets {
  return {
    server_url: 'https://api.day.app',
    device_key: '',
    sound: 'bell',
    group: 'ServiceCompass',
    url: '',
    method: 'POST',
    headers: '{}',
    mode: 'incoming',
    base_url: '',
    token: '',
    user_ids: '',
    verify_tls: true,
    service_scope: 'all',
    service_ids: [],
  }
}

async function load() {
  ;[channels.value, services.value] = await Promise.all([
    notificationsApi.channels(),
    servicesApi.list(),
  ])
}

function openChannel(channel?: NotificationChannel) {
  editingChannel.value = channel ?? null
  channelForm.value = channel
    ? { name: channel.name, channel_type: channel.channel_type, enabled: channel.enabled }
    : emptyChannel()
  secrets.value = channel ? secretsFromConfig(channel.config) : emptySecrets()
  channelModal.value = true
}

function secretsFromConfig(config: Record<string, unknown>): ChannelSecrets {
  const value = emptySecrets()
  if (typeof config.server_url === 'string') value.server_url = config.server_url
  if (typeof config.device_key === 'string') value.device_key = config.device_key
  if (typeof config.sound === 'string') value.sound = config.sound
  if (typeof config.group === 'string') value.group = config.group
  if (typeof config.url === 'string') value.url = config.url
  if (typeof config.method === 'string') value.method = config.method
  if (config.headers && typeof config.headers === 'object') {
    value.headers = JSON.stringify(config.headers, null, 2)
  }
  if (typeof config.mode === 'string') value.mode = config.mode
  if (typeof config.base_url === 'string') value.base_url = config.base_url
  if (typeof config.token === 'string') value.token = config.token
  if (Array.isArray(config.user_ids)) value.user_ids = config.user_ids.map(String).join(', ')
  if (typeof config.verify_tls === 'boolean') value.verify_tls = config.verify_tls
  if (Array.isArray(config.service_ids) && config.service_ids.length) {
    value.service_scope = 'selected'
    value.service_ids = config.service_ids.filter(
      (item): item is string => typeof item === 'string',
    )
  }
  return value
}

async function saveChannel() {
  if (!channelForm.value.name.trim()) return message.warning('请输入通道名称')
  const config = buildConfig()
  if (!config) return message.warning('请填写完整的通知配置')
  const input = { ...channelForm.value, config }
  if (editingChannel.value) await notificationsApi.updateChannel(editingChannel.value.id, input)
  else await notificationsApi.createChannel(input)
  channelModal.value = false
  message.success('通知通道已保存')
  await load()
}

function buildConfig(): Record<string, unknown> | undefined {
  const value = secrets.value
  const scoped = serviceScopeConfig()
  if (channelForm.value.channel_type === 'bark') {
    if (!value.device_key) return undefined
    return {
      server_url: value.server_url,
      device_key: value.device_key,
      sound: value.sound,
      group: value.group,
      ...scoped,
    }
  }
  if (channelForm.value.channel_type === 'webhook') {
    if (!value.url) return undefined
    let headers: Record<string, string>
    try {
      headers = JSON.parse(value.headers) as Record<string, string>
    } catch {
      message.error('Webhook Headers 必须是 JSON')
      return undefined
    }
    return { url: value.url, method: value.method, headers, ...scoped }
  }
  return buildSynologyConfig(scoped)
}

function buildSynologyConfig(scoped: Record<string, unknown>) {
  const value = secrets.value
  if (!value.base_url) return undefined
  const config: Record<string, unknown> = {
    mode: value.mode,
    base_url: value.base_url,
    token: value.token,
    verify_tls: value.verify_tls,
    ...scoped,
  }
  if (value.mode !== 'chatbot') return config
  const userIdTexts = value.user_ids
    .split(/[,\n，]/)
    .map((item) => item.trim())
    .filter(Boolean)
  if (!userIdTexts.length) {
    message.warning('Chatbot 模式必须填写至少一个用户 ID')
    return undefined
  }
  if (userIdTexts.some((item) => !/^\d+$/.test(item) || Number(item) <= 0)) {
    message.warning('Chatbot 用户 ID 必须是 Synology Chat 中的数字 user_id')
    return undefined
  }
  return { ...config, target_type: 'user', user_ids: userIdTexts.map(Number) }
}

function serviceScopeConfig() {
  if (secrets.value.service_scope !== 'selected') return {}
  return { service_ids: secrets.value.service_ids }
}

function selectAllServices() {
  secrets.value.service_scope = 'selected'
  secrets.value.service_ids = services.value.map((item) => item.id)
}

async function testChannel(channel: NotificationChannel) {
  const result = await notificationsApi.testChannel(channel.id)
  message.success(`测试发送成功 · HTTP ${result.status_code}`)
}

function removeChannel(channel: NotificationChannel) {
  dialog.warning({
    title: '删除通知通道',
    content: `确认删除 ${channel.name}？监控中的关联通知也会失效。`,
    positiveText: '删除',
    negativeText: '取消',
    maskClosable: false,
    onPositiveClick: async () => {
      await notificationsApi.removeChannel(channel.id)
      await load()
    },
  })
}

function scopeText(channel: NotificationChannel) {
  const ids = Array.isArray(channel.config.service_ids) ? channel.config.service_ids : []
  if (!ids.length) return '全部服务'
  return `${ids.length} 个服务`
}

onMounted(load)
</script>

<template>
  <header class="page-header">
    <div>
      <p>ALERT SIGNALS</p>
      <h1>通知通道</h1>
      <span>通道负责发送方式和服务作用域；每个监控单独选择是否通知。</span>
    </div>
    <NButton type="primary" @click="openChannel()">
      <template #icon><NIcon :component="Bell" /></template>
      添加通道
    </NButton>
  </header>

  <section class="channel-grid">
    <NCard v-for="channel in channels" :key="channel.id" size="small" class="channel-card">
      <div class="channel-title">
        <NIcon :component="Bell" class="channel-icon" />
        <div class="channel-main">
          <strong>{{ channel.name }}</strong>
          <small>{{ channel.channel_type === 'synology_chat' ? 'Synology Chat' : channel.channel_type }} · {{ scopeText(channel) }}</small>
        </div>
        <NTag :type="channel.enabled ? 'success' : 'warning'" size="small" :bordered="false">
          {{ channel.enabled ? '启用' : '停用' }}
        </NTag>
      </div>
      <NSpace size="small" class="channel-actions">
        <NButton size="small" @click="testChannel(channel)">
          <template #icon><NIcon :component="PlayerPlay" /></template>
          测试
        </NButton>
        <NButton quaternary circle size="small" @click="openChannel(channel)"><NIcon :component="Edit" /></NButton>
        <NButton quaternary circle size="small" @click="removeChannel(channel)"><NIcon :component="Trash" /></NButton>
      </NSpace>
    </NCard>
  </section>

  <NModal
    v-model:show="channelModal"
    preset="card"
    :title="editingChannel ? '编辑通知通道' : '添加通知通道'"
    class="notify-modal"
    :mask-closable="false"
  >
    <NForm label-placement="top">
      <div class="two-columns">
        <NFormItem label="名称"><NInput v-model:value="channelForm.name" /></NFormItem>
        <NFormItem label="类型">
          <NSelect
            v-model:value="channelForm.channel_type"
            :disabled="Boolean(editingChannel)"
            :options="[
              { label: 'Bark', value: 'bark' },
              { label: 'Webhook', value: 'webhook' },
              { label: 'Synology Chat', value: 'synology_chat' },
            ]"
          />
        </NFormItem>
      </div>

      <template v-if="channelForm.channel_type === 'bark'">
        <NFormItem label="服务器地址"><NInput v-model:value="secrets.server_url" /></NFormItem>
        <NFormItem label="Device Key"><NInput v-model:value="secrets.device_key" /></NFormItem>
      </template>
      <template v-else-if="channelForm.channel_type === 'webhook'">
        <NFormItem label="Webhook URL"><NInput v-model:value="secrets.url" /></NFormItem>
        <NFormItem label="Headers JSON">
          <NInput v-model:value="secrets.headers" type="textarea" placeholder='{"Authorization":"Bearer ..."}' />
        </NFormItem>
      </template>
      <template v-else>
        <NFormItem label="模式">
          <NSelect
            v-model:value="secrets.mode"
            :options="[
              { label: 'Incoming Webhook（发送到固定频道，推荐）', value: 'incoming' },
              { label: 'Chatbot（发送给指定用户）', value: 'chatbot' },
            ]"
          />
        </NFormItem>
        <NFormItem label="Chat 地址或完整 Webhook URL">
          <NInput v-model:value="secrets.base_url" placeholder="填写 NAS 地址或 DSM 生成的完整 URL" />
        </NFormItem>
        <NFormItem label="Token（完整 URL 已包含时可不填）">
          <NInput v-model:value="secrets.token" placeholder="填写 Synology Chat Token" />
        </NFormItem>
        <NFormItem v-if="secrets.mode === 'chatbot'" label="用户 ID（逗号或换行分隔）">
          <NInput v-model:value="secrets.user_ids" placeholder="例如：5, 12" />
        </NFormItem>
        <p v-if="secrets.mode === 'chatbot'" class="form-tip">
          Chatbot 只能发送给 Bot 可见的数字 user_id；如果要发到频道，请在 Synology Chat 创建 Incoming Webhook 并选择 Incoming 模式。
        </p>
        <NFormItem label="TLS 校验"><NSwitch v-model:value="secrets.verify_tls" /></NFormItem>
      </template>

      <NCard size="small" class="scope-card">
        <template #header>生效服务</template>
        <div class="scope-row">
          <NSelect
            v-model:value="secrets.service_scope"
            :options="[
              { label: '全部服务', value: 'all' },
              { label: '指定服务', value: 'selected' },
            ]"
          />
          <NButton v-if="secrets.service_scope === 'selected'" @click="selectAllServices">
            <template #icon><NIcon :component="Check" /></template>
            全选
          </NButton>
        </div>
        <NSelect
          v-if="secrets.service_scope === 'selected'"
          v-model:value="secrets.service_ids"
          :options="serviceOptions"
          multiple
          filterable
          placeholder="选择生效服务"
          class="service-select"
        />
      </NCard>

      <div class="footer-row">
        <label><NSwitch v-model:value="channelForm.enabled" /> 启用通道</label>
        <NButton type="primary" @click="saveChannel">保存通道</NButton>
      </div>
    </NForm>
  </NModal>
</template>

<style scoped>
.page-header { display: flex; align-items: end; justify-content: space-between; gap: 2rem; margin-bottom: 1.5rem; }
.page-header p { margin: 0; color: #fbbf24; font-family: "IBM Plex Mono", monospace; font-size: 0.68rem; letter-spacing: 0.2em; }
.page-header h1 { margin: 0.35rem 0; font-size: 2.35rem; }
.page-header span, .channel-title small { color: var(--sc-muted); }
.channel-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(16.5rem, 1fr)); gap: 0.7rem; margin-bottom: 1rem; }
.channel-card :deep(.n-card__content) { padding: 0.75rem; }
.channel-title { display: flex; align-items: center; gap: 0.55rem; margin-bottom: 0.55rem; min-width: 0; }
.channel-icon { flex: 0 0 auto; font-size: 1.05rem; }
.channel-main { display: grid; flex: 1; min-width: 0; font-size: 0.86rem; }
.channel-main strong, .channel-main small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.channel-main small { font-size: 0.72rem; }
.channel-actions { flex-wrap: nowrap; }
.notify-modal { width: min(48rem, calc(100vw - 2rem)); }
.two-columns { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; }
.scope-card { margin: 0.5rem 0 1rem; background: var(--sc-card); }
.scope-row, .footer-row { display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; }
.service-select { margin-top: 0.75rem; }
.footer-row label { display: flex; align-items: center; gap: 0.45rem; color: var(--sc-muted); }
.form-tip { margin: -0.35rem 0 0.85rem; color: var(--sc-muted); font-size: 0.78rem; line-height: 1.55; }
@media (max-width: 760px) { .page-header { align-items: flex-start; flex-direction: column; } .two-columns { grid-template-columns: 1fr; } .scope-row, .footer-row { align-items: stretch; flex-direction: column; } }
</style>
