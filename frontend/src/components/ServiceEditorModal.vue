<script setup lang="ts">
import { BrandDocker, HeartRateMonitor, Plus, Refresh } from '@vicons/tabler'
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
  useMessage,
} from 'naive-ui'
import { computed, ref, watch } from 'vue'
import { dockerApi } from '../api/docker'
import { groupsApi } from '../api/groups'
import { notificationsApi } from '../api/notifications'
import type {
  DockerCandidate,
  DockerEndpoint,
  Group,
  MonitorInput,
  NotificationChannel,
  ServiceInput,
  Space,
  UrlMode,
} from '../types'
import DockerEndpointModal from './DockerEndpointModal.vue'
import IconPicker from './IconPicker.vue'
import MonitorForm from './MonitorForm.vue'

const props = defineProps<{ groups: Group[]; spaces: Space[]; editing: boolean; title?: string }>()
const emit = defineEmits<{ save: []; 'group-created': [group: Group] }>()
const show = defineModel<boolean>('show', { required: true })
const form = defineModel<ServiceInput>('form', { required: true })
const monitor = defineModel<MonitorInput>('monitor', { required: true })
const endpoints = ref<DockerEndpoint[]>([])
const candidates = ref<DockerCandidate[]>([])
const notificationChannels = ref<NotificationChannel[]>([])
const dockerEnabled = ref(false)
const scanning = ref(false)
const selectedCandidate = ref<string | null>(null)
const localGroups = ref<Group[]>([])
const localSpaces = ref<Space[]>([])
const addingGroup = ref(false)
const newGroupName = ref('')
const newGroupSpaceId = ref('')
const creatingGroup = ref(false)
const endpointModal = ref(false)
const message = useMessage()

const endpointOptions = computed(() =>
  endpoints.value.map((item) => ({ label: item.name, value: item.id })),
)
const candidateOptions = computed(() => {
  const options = candidates.value.map((item) => ({
    label: `${item.suggested_name} · ${item.image || item.container_name}`,
    value: item.container_id,
  }))
  const current = form.value.docker_container_id
  if (current && !options.some((item) => item.value === current)) {
    options.unshift({
      label: `${form.value.docker_name || '已关联容器'} · ${form.value.docker_image || current.slice(0, 12)}`,
      value: current,
    })
  }
  return options
})
const groupOptions = computed(() => [
  { label: '未分组', value: '' },
  ...localGroups.value.map((item) => ({
    label: `${spaceName(item.space_id)} / ${item.name}`,
    value: item.id,
  })),
])
const spaceOptions = computed(() =>
  localSpaces.value.map((item) => ({ label: item.name, value: item.id })),
)
const monitorUrl = computed(() => {
  if (monitor.value.target_url_mode === 'custom') return monitor.value.target_url || ''
  if (monitor.value.target_url_mode === 'local')
    return form.value.local_url || form.value.public_url || ''
  return form.value.public_url || form.value.local_url || ''
})
const canNotifyCertificate = computed(() =>
  monitorUrl.value.trim().toLowerCase().startsWith('https://'),
)

watch(show, async (value) => {
  if (!value) return
  dockerEnabled.value = Boolean(form.value.docker_endpoint_id)
  selectedCandidate.value = form.value.docker_container_id || null
  localGroups.value = [...props.groups]
  localSpaces.value = [...props.spaces]
  newGroupSpaceId.value = localSpaces.value[0]?.id || ''
  addingGroup.value = false
  newGroupName.value = ''
  ;[endpoints.value, notificationChannels.value] = await Promise.all([
    dockerApi.endpoints(),
    notificationsApi.channels(),
  ])
  applyDefaultNotification()
})

watch(canNotifyCertificate, (value) => {
  if (!value) form.value.cert_expiry_notify = false
})

async function createGroup() {
  const name = newGroupName.value.trim()
  if (!name) return message.warning('请输入分组名称')
  if (!newGroupSpaceId.value) return message.warning('请先创建空间')
  creatingGroup.value = true
  try {
    const group = await groupsApi.create({
      space_id: newGroupSpaceId.value,
      name,
      sort_order: localGroups.value.length,
    })
    localGroups.value.push(group)
    form.value.group_id = group.id
    addingGroup.value = false
    newGroupName.value = ''
    emit('group-created', group)
    message.success('分组已创建并选中')
  } finally {
    creatingGroup.value = false
  }
}

function spaceName(id: string) {
  return localSpaces.value.find((item) => item.id === id)?.name || '默认空间'
}

watch(dockerEnabled, (value) => {
  if (value) return
  clearDockerSelection(true)
  candidates.value = []
  selectedCandidate.value = null
})

watch(
  () => form.value.docker_endpoint_id,
  (value, oldValue) => {
    if (!show.value || !oldValue || value === oldValue) return
    clearDockerSelection(false)
    candidates.value = []
    selectedCandidate.value = null
  },
)

function clearDockerSelection(clearEndpoint: boolean) {
  if (clearEndpoint) form.value.docker_endpoint_id = null
  form.value.docker_container_id = null
  form.value.docker_name = null
  form.value.docker_image = null
  form.value.docker_compose_project = null
  form.value.docker_compose_service = null
}

async function scan() {
  if (!form.value.docker_endpoint_id) return message.warning('请先选择 Docker 端点')
  scanning.value = true
  try {
    candidates.value = await dockerApi.scan(form.value.docker_endpoint_id)
    message.success(`发现 ${candidates.value.length} 个候选服务`)
  } finally {
    scanning.value = false
  }
}

function selectCandidate(containerId: string | null) {
  selectedCandidate.value = containerId
  const candidate = candidates.value.find((item) => item.container_id === containerId)
  if (!candidate) return
  form.value.docker_endpoint_id = candidate.endpoint_id
  form.value.docker_container_id = candidate.container_id
  form.value.docker_name = candidate.container_name
  form.value.docker_image = candidate.image
  form.value.docker_compose_project = candidate.compose_project
  form.value.docker_compose_service = candidate.compose_service
  form.value.name ||= candidate.suggested_name
  form.value.local_url ||= candidate.local_url
  form.value.public_url ||= candidate.public_url
  if (!form.value.icon_value && candidate.suggested_icon) {
    form.value.icon_type = 'selfhst'
    form.value.icon_value = candidate.suggested_icon
  }
}

function endpointSaved(endpoint: DockerEndpoint) {
  const index = endpoints.value.findIndex((item) => item.id === endpoint.id)
  if (index >= 0) endpoints.value[index] = endpoint
  else endpoints.value.push(endpoint)
  form.value.docker_endpoint_id = endpoint.id
}

function applyDefaultNotification() {
  monitor.value.notify_on_down = true
  monitor.value.notify_on_recovery = true
  monitor.value.notify_on_warning = true
  if (props.editing || notificationChannels.value.length !== 1 || monitor.value.notify_enabled)
    return
  monitor.value.notify_enabled = true
  monitor.value.notification_channel_ids = [notificationChannels.value[0].id]
}
</script>

<template>
  <NModal
    v-model:show="show"
    preset="card"
    :title="title || (editing ? '编辑服务' : '添加服务')"
    class="modal-wide"
    :mask-closable="false"
  >
    <NForm label-placement="top" size="small">
      <div class="form-grid">
        <NFormItem label="服务名称" class="span-2"><NInput v-model:value="form.name" placeholder="例如：Home Assistant" /></NFormItem>
        <NFormItem label="外网地址（可选）"><NInput v-model:value="form.public_url" placeholder="https://service.example.com" /></NFormItem>
        <NFormItem label="内网地址（可选）"><NInput v-model:value="form.local_url" placeholder="http://192.168.1.10:8080" /></NFormItem>
        <NFormItem label="分组（可选）">
          <div class="select-with-action">
            <NSelect v-model:value="form.group_id" :options="groupOptions" placeholder="选择分组或保持未分组" />
            <NButton title="新建分组" @click="addingGroup = !addingGroup"><NIcon :component="Plus" /></NButton>
          </div>
          <div v-if="addingGroup" class="inline-create">
            <NSelect v-model:value="newGroupSpaceId" :options="spaceOptions" placeholder="选择空间" />
            <NInput v-model:value="newGroupName" placeholder="输入新分组名称" @keyup.enter="createGroup" />
            <NButton type="primary" :loading="creatingGroup" @click="createGroup">创建</NButton>
          </div>
        </NFormItem>
        <NFormItem label="默认访问">
          <NSelect v-model:value="form.preferred_url_mode" :options="[{ label: '外网', value: 'public' as UrlMode }, { label: '内网', value: 'local' as UrlMode }]" />
        </NFormItem>
        <NFormItem label="说明（可选）" class="span-2"><NInput v-model:value="form.description" /></NFormItem>
        <NFormItem label="服务图标" class="span-2">
          <IconPicker :name="form.name" :icon-type="form.icon_type" :icon-value="form.icon_value" :service-url="form.public_url || form.local_url" @update:icon-type="form.icon_type = $event" @update:icon-value="form.icon_value = $event" />
        </NFormItem>
      </div>

      <NCard size="small" class="option-card">
        <template #header><span class="option-title"><NIcon :component="BrandDocker" />关联 Docker（可选）</span></template>
        <template #header-extra><NSwitch v-model:value="dockerEnabled" /></template>
        <div v-if="dockerEnabled" class="docker-row">
          <NSelect v-model:value="form.docker_endpoint_id" :options="endpointOptions" placeholder="选择 Docker 端点" />
          <NButton title="添加 Docker Endpoint" @click="endpointModal = true"><NIcon :component="Plus" /></NButton>
          <NButton :loading="scanning" @click="scan"><template #icon><NIcon :component="Refresh" /></template>扫描</NButton>
          <NSelect class="candidate-select" :value="selectedCandidate" :options="candidateOptions" filterable clearable placeholder="输入名称、容器或镜像筛选候选服务" @update:value="selectCandidate" />
          <small class="docker-note">选择容器后会自动创建 Docker 状态监控，不能单独关闭。</small>
        </div>
      </NCard>

      <NCard size="small" class="option-card">
        <template #header><span class="option-title"><NIcon :component="HeartRateMonitor" />服务监控（可选）</span></template>
        <template #header-extra><NSwitch v-model:value="form.create_monitor" /></template>
        <MonitorForm
          v-if="form.create_monitor"
          v-model="monitor"
          :services="[]"
          :show-identity="false"
          show-notification
          :notification-channels="notificationChannels"
          :allowed-types="['http', 'http_keyword']"
        />
        <label v-if="form.create_monitor" class="cert-toggle" :class="{ disabled: !canNotifyCertificate }">
          <NSwitch v-model:value="form.cert_expiry_notify" :disabled="!canNotifyCertificate" />
          HTTPS 证书到期时通知
          <small>提前天数在设置中统一配置</small>
        </label>
      </NCard>

      <div class="footer-row">
        <span>显示服务 <NSwitch v-model:value="form.enabled" size="small" /></span>
        <NSpace><NButton @click="show = false">取消</NButton><NButton type="primary" @click="emit('save')">保存服务</NButton></NSpace>
      </div>
    </NForm>
  </NModal>
  <DockerEndpointModal v-model:show="endpointModal" @saved="endpointSaved" />
</template>

<style scoped>
.form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 0 0.8rem; }
.span-2 { grid-column: span 2; }
.option-card { margin-top: 0.75rem; background: var(--sc-card); }
.option-title, .footer-row span { display: flex; align-items: center; gap: 0.45rem; }
.select-with-action, .inline-create { display: flex; width: 100%; gap: 0.5rem; }
.inline-create { display: grid; grid-template-columns: 10rem 1fr auto; margin-top: 0.5rem; }
.docker-row { display: grid; grid-template-columns: 1fr auto auto; gap: 0.6rem; }
.candidate-select, .docker-note { grid-column: span 3; }
.docker-note { color: var(--sc-muted); }
.cert-toggle { display: flex; align-items: center; gap: 0.5rem; margin-top: 0.4rem; color: var(--sc-muted); }
.cert-toggle small { margin-left: 0.2rem; color: var(--sc-subtle); }
.cert-toggle.disabled { opacity: 0.55; }
.footer-row { display: flex; align-items: center; justify-content: space-between; margin-top: 1rem; }
@media (max-width: 620px) { .form-grid, .monitor-row, .inline-create { grid-template-columns: 1fr; } .span-2 { grid-column: auto; } }
</style>
