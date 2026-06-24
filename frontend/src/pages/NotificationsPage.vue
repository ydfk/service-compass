<script setup lang="ts">
import { Bell, Edit, PlayerPlay, Plus, Trash } from '@vicons/tabler'
import {
  NButton,
  NCard,
  NDataTable,
  NForm,
  NFormItem,
  NIcon,
  NInput,
  NInputNumber,
  NModal,
  NSelect,
  NSpace,
  NSwitch,
  NTag,
  useDialog,
  useMessage,
  type DataTableColumns,
} from 'naive-ui'
import { h, onMounted, ref } from 'vue'
import { monitorsApi } from '../api/monitors'
import { notificationsApi, type NotificationChannelInput } from '../api/notifications'
import type {
  Monitor,
  NotificationChannel,
  NotificationRule,
  NotificationRuleInput,
} from '../types'

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
  target_type: string
  channel_id: string
  user_ids: string
  verify_tls: boolean
}

const channels = ref<NotificationChannel[]>([])
const rules = ref<NotificationRule[]>([])
const monitors = ref<Monitor[]>([])
const editingChannel = ref<NotificationChannel | null>(null)
const channelModal = ref(false)
const ruleModal = ref(false)
const channelForm = ref<NotificationChannelInput>(emptyChannel())
const secrets = ref<ChannelSecrets>(emptySecrets())
const ruleForm = ref<NotificationRuleInput>(emptyRule())
const message = useMessage()
const dialog = useDialog()

const ruleColumns: DataTableColumns<NotificationRule> = [
  { title: '监控', key: 'monitor_id', render: (row) => monitorName(row.monitor_id) },
  { title: '通道', key: 'channel_id', render: (row) => channelName(row.channel_id) },
  { title: '离线', key: 'notify_on_down', render: (row) => (row.notify_on_down ? '通知' : '—') },
  {
    title: '恢复',
    key: 'notify_on_recovery',
    render: (row) => (row.notify_on_recovery ? '通知' : '—'),
  },
  {
    title: '警告',
    key: 'notify_on_warning',
    render: (row) => (row.notify_on_warning ? '通知' : '—'),
  },
  { title: '冷却', key: 'cooldown_sec', render: (row) => `${row.cooldown_sec} 秒` },
  {
    title: '',
    key: 'action',
    render: (row) =>
      h(
        NButton,
        { quaternary: true, size: 'small', onClick: () => removeRule(row) },
        { icon: () => h(NIcon, { component: Trash }) },
      ),
  },
]

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
    target_type: 'channel',
    channel_id: '',
    user_ids: '',
    verify_tls: true,
  }
}

function emptyRule(): NotificationRuleInput {
  return {
    monitor_id: null,
    channel_id: '',
    notify_on_down: true,
    notify_on_recovery: true,
    notify_on_warning: true,
    cooldown_sec: 300,
    enabled: true,
  }
}

async function load() {
  ;[channels.value, rules.value, monitors.value] = await Promise.all([
    notificationsApi.channels(),
    notificationsApi.rules(),
    monitorsApi.list(),
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
  if (typeof config.target_type === 'string') value.target_type = config.target_type
  if (typeof config.channel_id === 'string' || typeof config.channel_id === 'number') {
    value.channel_id = String(config.channel_id)
  }
  if (Array.isArray(config.user_ids)) value.user_ids = config.user_ids.join(', ')
  if (typeof config.verify_tls === 'boolean') value.verify_tls = config.verify_tls
  if (value.mode === 'chatbot' && !value.channel_id && !value.user_ids) value.mode = 'incoming'
  return value
}

async function saveChannel() {
  if (!channelForm.value.name) return message.warning('请输入通道名称')
  const config = buildConfig()
  if (!config) return message.warning('请填写完整的通知配置')
  const input = { ...channelForm.value, ...(config ? { config } : {}) }
  if (editingChannel.value) await notificationsApi.updateChannel(editingChannel.value.id, input)
  else await notificationsApi.createChannel(input)
  channelModal.value = false
  message.success('通知通道已保存')
  await load()
}

function buildConfig(): Record<string, unknown> | undefined {
  const value = secrets.value
  if (channelForm.value.channel_type === 'bark') {
    if (!value.device_key) return undefined
    return {
      server_url: value.server_url,
      device_key: value.device_key,
      sound: value.sound,
      group: value.group,
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
    return { url: value.url, method: value.method, headers }
  }
  if (!value.base_url) return undefined
  const config: Record<string, unknown> = {
    mode: value.mode,
    base_url: value.base_url,
    token: value.token,
    verify_tls: value.verify_tls,
  }
  if (value.mode !== 'chatbot') return config
  if (value.target_type === 'channel') {
    if (!value.channel_id.trim()) {
      message.warning('chatbot 模式必须填写频道 ID')
      return undefined
    }
    return { ...config, target_type: 'channel', channel_id: value.channel_id.trim() }
  }
  const userIds = value.user_ids
    .split(',')
    .map((item) => Number(item.trim()))
    .filter(Number.isFinite)
  if (!userIds.length) {
    message.warning('chatbot 模式必须填写至少一个用户 ID')
    return undefined
  }
  return { ...config, target_type: 'user', user_ids: userIds }
}

async function testChannel(channel: NotificationChannel) {
  const result = await notificationsApi.testChannel(channel.id)
  message.success(`测试发送成功 · HTTP ${result.status_code}`)
}

function removeChannel(channel: NotificationChannel) {
  dialog.warning({
    title: '删除通知通道',
    content: `确认删除 ${channel.name}？关联规则也会删除。`,
    positiveText: '删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      await notificationsApi.removeChannel(channel.id)
      await load()
    },
  })
}

function openRule() {
  ruleForm.value = { ...emptyRule(), channel_id: channels.value[0]?.id || '' }
  ruleModal.value = true
}

async function saveRule() {
  if (!ruleForm.value.channel_id) return message.warning('请选择通知通道')
  await notificationsApi.createRule(ruleForm.value)
  ruleModal.value = false
  await load()
}

function removeRule(rule: NotificationRule) {
  dialog.warning({
    title: '删除规则',
    content: '确认删除这条通知规则？',
    positiveText: '删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      await notificationsApi.removeRule(rule.id)
      await load()
    },
  })
}

function monitorName(id?: string | null) {
  return id ? monitors.value.find((item) => item.id === id)?.name || '已删除监控' : '全部监控'
}

function channelName(id: string) {
  return channels.value.find((item) => item.id === id)?.name || '已删除通道'
}

onMounted(load)
</script>

<template>
  <header class="page-header"><div><p>ALERT SIGNALS</p><h1>通知通道</h1><span>编辑时可直接查看和修改通道配置。</span></div><NSpace><NButton :disabled="channels.length === 0" @click="openRule()">新建规则</NButton><NButton type="primary" @click="openChannel()"><template #icon><NIcon :component="Plus" /></template>添加通道</NButton></NSpace></header>
  <section class="channel-grid"><NCard v-for="channel in channels" :key="channel.id" size="small"><div class="channel-title"><NIcon :component="Bell" /><div><strong>{{ channel.name }}</strong><small>{{ channel.channel_type === 'synology_chat' ? 'Synology Chat 机器人' : channel.channel_type }}</small></div><NTag type="success" size="small" :bordered="false">已配置</NTag></div><NSpace><NButton size="small" @click="testChannel(channel)"><template #icon><NIcon :component="PlayerPlay" /></template>测试发送</NButton><NButton quaternary circle size="small" @click="openChannel(channel)"><NIcon :component="Edit" /></NButton><NButton quaternary circle size="small" @click="removeChannel(channel)"><NIcon :component="Trash" /></NButton></NSpace></NCard></section>
  <NDataTable :columns="ruleColumns" :data="rules" :row-key="(row: NotificationRule) => row.id" />

  <NModal v-model:show="channelModal" preset="card" :title="editingChannel ? '编辑通知通道' : '添加通知通道'" class="notify-modal"><NForm label-placement="top"><div class="two-columns"><NFormItem label="名称"><NInput v-model:value="channelForm.name" /></NFormItem><NFormItem label="类型"><NSelect v-model:value="channelForm.channel_type" :disabled="Boolean(editingChannel)" :options="[{ label: 'Bark', value: 'bark' }, { label: 'Webhook', value: 'webhook' }, { label: 'Synology Chat 机器人', value: 'synology_chat' }]" /></NFormItem></div><template v-if="channelForm.channel_type === 'bark'"><NFormItem label="服务器地址"><NInput v-model:value="secrets.server_url" /></NFormItem><NFormItem label="Device Key"><NInput v-model:value="secrets.device_key" /></NFormItem></template><template v-else-if="channelForm.channel_type === 'webhook'"><NFormItem label="Webhook URL"><NInput v-model:value="secrets.url" /></NFormItem><NFormItem label="Headers JSON"><NInput v-model:value="secrets.headers" type="textarea" placeholder='{"Authorization":"Bearer ..."}' /></NFormItem></template><template v-else><NFormItem label="模式"><NSelect v-model:value="secrets.mode" :options="[{ label: 'Incoming Webhook（推荐）', value: 'incoming' }, { label: 'Chatbot（指定频道或用户）', value: 'chatbot' }]" /></NFormItem><NFormItem label="Chat 地址或完整 Webhook URL"><NInput v-model:value="secrets.base_url" placeholder="填写 NAS 地址或 DSM 生成的完整 URL" /></NFormItem><NFormItem label="Token（完整 URL 已包含时可不填）"><NInput v-model:value="secrets.token" placeholder="填写 Synology Chat Token" /></NFormItem><div v-if="secrets.mode === 'chatbot'" class="two-columns"><NFormItem label="发送目标"><NSelect v-model:value="secrets.target_type" :options="[{ label: '频道', value: 'channel' }, { label: '用户', value: 'user' }]" /></NFormItem><NFormItem v-if="secrets.target_type === 'channel'" label="频道 ID"><NInput v-model:value="secrets.channel_id" placeholder="填写频道 ID" /></NFormItem><NFormItem v-if="secrets.target_type === 'user'" label="用户 ID（逗号分隔）"><NInput v-model:value="secrets.user_ids" placeholder="例如：12, 15" /></NFormItem></div><NFormItem label="TLS 校验"><NSwitch v-model:value="secrets.verify_tls" /></NFormItem></template><NButton type="primary" block @click="saveChannel">保存通道</NButton></NForm></NModal>
  <NModal v-model:show="ruleModal" preset="card" title="新建通知规则" class="rule-modal"><NForm label-placement="top"><NFormItem label="监控范围"><NSelect :value="ruleForm.monitor_id || ''" :options="[{ label: '全部监控', value: '' }, ...monitors.map((item) => ({ label: item.name, value: item.id }))]" @update:value="ruleForm.monitor_id = $event || null" /></NFormItem><NFormItem label="通知通道"><NSelect v-model:value="ruleForm.channel_id" :options="channels.map((item) => ({ label: item.name, value: item.id }))" /></NFormItem><NFormItem label="冷却时间（秒）"><NInputNumber v-model:value="ruleForm.cooldown_sec" :min="0" /></NFormItem><div class="switches"><label><NSwitch v-model:value="ruleForm.notify_on_down" />离线</label><label><NSwitch v-model:value="ruleForm.notify_on_recovery" />恢复</label><label><NSwitch v-model:value="ruleForm.notify_on_warning" />警告</label></div><NButton type="primary" block @click="saveRule">保存规则</NButton></NForm></NModal>
</template>

<style scoped>
.page-header { display: flex; align-items: end; justify-content: space-between; gap: 2rem; margin-bottom: 1.5rem; }
.page-header p { margin: 0; color: #fbbf24; font-family: "IBM Plex Mono", monospace; font-size: 0.68rem; letter-spacing: 0.2em; }
.page-header h1 { margin: 0.35rem 0; font-size: 2.35rem; }
.page-header span, .channel-title small { color: #75859b; }
.channel-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(18rem, 1fr)); gap: 1rem; margin-bottom: 1rem; }
.channel-title { display: flex; align-items: center; gap: 0.8rem; margin-bottom: 1rem; font-size: 1.35rem; }
.channel-title div { display: grid; flex: 1; font-size: 0.9rem; }
.notify-modal { width: min(48rem, calc(100vw - 2rem)); }
.rule-modal { width: min(32rem, calc(100vw - 2rem)); }
.two-columns { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; }
.switches { display: flex; gap: 1rem; margin-bottom: 1.5rem; }
.switches label { display: flex; align-items: center; gap: 0.4rem; }
@media (max-width: 760px) { .page-header { align-items: flex-start; flex-direction: column; } .two-columns { grid-template-columns: 1fr; } }
</style>
