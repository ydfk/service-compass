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
import type {
  DockerCandidate,
  DockerEndpoint,
  Group,
  MonitorInput,
  ServiceInput,
  UrlMode,
} from '../types'
import IconPicker from './IconPicker.vue'
import MonitorForm from './MonitorForm.vue'

const props = defineProps<{ groups: Group[]; editing: boolean; title?: string }>()
const emit = defineEmits<{ save: []; 'group-created': [group: Group] }>()
const show = defineModel<boolean>('show', { required: true })
const form = defineModel<ServiceInput>('form', { required: true })
const monitor = defineModel<MonitorInput>('monitor', { required: true })
const endpoints = ref<DockerEndpoint[]>([])
const candidates = ref<DockerCandidate[]>([])
const dockerEnabled = ref(false)
const scanning = ref(false)
const selectedCandidate = ref<string | null>(null)
const localGroups = ref<Group[]>([])
const addingGroup = ref(false)
const newGroupName = ref('')
const creatingGroup = ref(false)
const message = useMessage()

const endpointOptions = computed(() =>
  endpoints.value.map((item) => ({ label: item.name, value: item.id })),
)
const candidateOptions = computed(() =>
  candidates.value.map((item) => ({
    label: `${item.suggested_name} · ${item.image || item.container_name}`,
    value: item.container_id,
  })),
)
const groupOptions = computed(() => [
  { label: '未分组', value: '' },
  ...localGroups.value.map((item) => ({ label: item.name, value: item.id })),
])

watch(show, async (value) => {
  if (!value) return
  dockerEnabled.value = Boolean(form.value.docker_endpoint_id)
  selectedCandidate.value = form.value.docker_container_id || null
  localGroups.value = [...props.groups]
  addingGroup.value = false
  newGroupName.value = ''
  endpoints.value = await dockerApi.endpoints()
})

async function createGroup() {
  const name = newGroupName.value.trim()
  if (!name) return message.warning('请输入分组名称')
  creatingGroup.value = true
  try {
    const group = await groupsApi.create({ name, sort_order: localGroups.value.length })
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

watch(dockerEnabled, (value) => {
  if (value) return
  form.value.docker_endpoint_id = null
  form.value.docker_container_id = null
  form.value.docker_name = null
  form.value.docker_image = null
  form.value.docker_compose_project = null
  form.value.docker_compose_service = null
  candidates.value = []
  selectedCandidate.value = null
})

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
</script>

<template>
  <NModal v-model:show="show" preset="card" :title="title || (editing ? '编辑服务' : '添加服务')" class="modal-wide">
    <NForm label-placement="top" size="small">
      <div class="form-grid">
        <NFormItem label="服务名称" class="span-2"><NInput v-model:value="form.name" placeholder="例如：Home Assistant" /></NFormItem>
        <NFormItem label="外网地址"><NInput v-model:value="form.public_url" placeholder="https://service.example.com" /></NFormItem>
        <NFormItem label="内网地址（可选）"><NInput v-model:value="form.local_url" placeholder="http://192.168.1.10:8080" /></NFormItem>
        <NFormItem label="分组（可选）">
          <div class="select-with-action">
            <NSelect v-model:value="form.group_id" :options="groupOptions" placeholder="选择分组或保持未分组" />
            <NButton title="新建分组" @click="addingGroup = !addingGroup"><NIcon :component="Plus" /></NButton>
          </div>
          <div v-if="addingGroup" class="inline-create">
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
          <NButton :loading="scanning" @click="scan"><template #icon><NIcon :component="Refresh" /></template>扫描</NButton>
          <NSelect class="candidate-select" :value="selectedCandidate" :options="candidateOptions" placeholder="选择候选容器，仅关联所选项" @update:value="selectCandidate" />
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
          :allowed-types="['http', 'http_keyword']"
        />
      </NCard>

      <div class="footer-row">
        <span>显示服务 <NSwitch v-model:value="form.enabled" size="small" /></span>
        <NSpace><NButton @click="show = false">取消</NButton><NButton type="primary" @click="emit('save')">保存服务</NButton></NSpace>
      </div>
    </NForm>
  </NModal>
</template>

<style scoped>
.form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 0 0.8rem; }
.span-2 { grid-column: span 2; }
.option-card { margin-top: 0.75rem; background: rgb(8 13 23 / 56%); }
.option-title, .footer-row span { display: flex; align-items: center; gap: 0.45rem; }
.select-with-action, .inline-create { display: flex; width: 100%; gap: 0.5rem; }
.inline-create { margin-top: 0.5rem; }
.docker-row { display: grid; grid-template-columns: 1fr auto; gap: 0.6rem; }
.candidate-select { grid-column: span 2; }
.docker-note { grid-column: span 2; color: #6f8098; }
.footer-row { display: flex; align-items: center; justify-content: space-between; margin-top: 1rem; }
@media (max-width: 620px) { .form-grid, .monitor-row { grid-template-columns: 1fr; } .span-2 { grid-column: auto; } }
</style>
