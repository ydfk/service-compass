<script setup lang="ts">
import { BrandDocker, ChartRadar, Edit, PlugConnected, Plus, Trash } from '@vicons/tabler'
import {
  NAlert,
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
import { onMounted, ref } from 'vue'
import { RouterLink } from 'vue-router'
import { dockerApi } from '../api/docker'
import DockerCandidateTable from '../components/DockerCandidateTable.vue'
import type { DockerCandidate, DockerEndpoint, DockerEndpointInput } from '../types'

const endpoints = ref<DockerEndpoint[]>([])
const candidates = ref<DockerCandidate[]>([])
const editing = ref<DockerEndpoint | null>(null)
const endpointModal = ref(false)
const scanning = ref(false)
const endpointForm = ref<DockerEndpointInput>(emptyEndpoint())
const message = useMessage()
const dialog = useDialog()

function emptyEndpoint(): DockerEndpointInput {
  return {
    name: '本机 Docker',
    endpoint_type: 'local_socket',
    endpoint_url: 'unix:///var/run/docker.sock',
    tls_enabled: false,
    tls_ca: '',
    tls_cert: '',
    tls_key: '',
    lan_host: '',
    public_host_hint: '',
    enabled: true,
  }
}

async function load() {
  endpoints.value = await dockerApi.endpoints()
}

function openEndpoint(endpoint?: DockerEndpoint) {
  editing.value = endpoint ?? null
  endpointForm.value = endpoint
    ? {
        name: endpoint.name,
        endpoint_type: endpoint.endpoint_type,
        endpoint_url: endpoint.endpoint_url,
        tls_enabled: endpoint.tls_enabled,
        tls_ca: '',
        tls_cert: '',
        tls_key: '',
        lan_host: endpoint.lan_host,
        public_host_hint: endpoint.public_host_hint,
        enabled: endpoint.enabled,
      }
    : emptyEndpoint()
  endpointModal.value = true
}

async function saveEndpoint() {
  if (!endpointForm.value.name || !endpointForm.value.endpoint_url) {
    return message.warning('请填写名称与 Endpoint 地址')
  }
  if (editing.value) await dockerApi.updateEndpoint(editing.value.id, endpointForm.value)
  else await dockerApi.createEndpoint(endpointForm.value)
  endpointModal.value = false
  message.success('Docker Endpoint 已保存')
  await load()
}

async function testEndpoint(endpoint: DockerEndpoint) {
  await dockerApi.testEndpoint(endpoint.id)
  message.success(`${endpoint.name} 连接成功`)
}

async function scan(endpoint: DockerEndpoint) {
  scanning.value = true
  try {
    candidates.value = await dockerApi.scan(endpoint.id)
    message.success(`发现 ${candidates.value.length} 个候选容器，未自动创建任何服务`)
  } finally {
    scanning.value = false
  }
}

function removeEndpoint(endpoint: DockerEndpoint) {
  dialog.warning({
    title: '删除 Endpoint',
    content: `删除 ${endpoint.name} 及其扫描缓存？已添加的服务不会被删除。`,
    positiveText: '删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      await dockerApi.removeEndpoint(endpoint.id)
      await load()
    },
  })
}

onMounted(load)
</script>

<template>
  <header class="page-header">
    <div>
      <p>DOCKER RADAR</p>
      <h1>Docker 辅助发现</h1>
      <span>扫描只生成候选；采用候选请回到“添加服务”统一完成关联。</span>
    </div>
    <NButton type="primary" @click="openEndpoint()">
      <template #icon><NIcon :component="Plus" /></template>添加 Endpoint
    </NButton>
  </header>
  <NAlert type="warning" :bordered="false" class="risk">
    Docker Socket 即使只读挂载也拥有很高权限。只在可信内网使用，远程 API 应启用 TLS。
  </NAlert>
  <section class="endpoint-grid">
    <NCard v-for="endpoint in endpoints" :key="endpoint.id" size="small">
      <div class="endpoint-title">
        <NIcon :component="BrandDocker" />
        <div><strong>{{ endpoint.name }}</strong><small>{{ endpoint.endpoint_url }}</small></div>
        <NTag size="small" :type="endpoint.tls_enabled ? 'success' : 'default'">{{ endpoint.tls_enabled ? 'TLS' : endpoint.endpoint_type }}</NTag>
      </div>
      <NSpace>
        <NButton size="small" @click="testEndpoint(endpoint)"><template #icon><NIcon :component="PlugConnected" /></template>测试</NButton>
        <NButton size="small" type="primary" :loading="scanning" @click="scan(endpoint)"><template #icon><NIcon :component="ChartRadar" /></template>扫描候选</NButton>
        <NButton size="small" quaternary @click="openEndpoint(endpoint)"><NIcon :component="Edit" /></NButton>
        <NButton size="small" quaternary @click="removeEndpoint(endpoint)"><NIcon :component="Trash" /></NButton>
      </NSpace>
    </NCard>
  </section>
  <NAlert v-if="candidates.length" type="info" :bordered="false" class="candidate-tip">
    已生成候选列表。<RouterLink to="/admin/services">前往服务页面添加并关联</RouterLink>
  </NAlert>
  <DockerCandidateTable :candidates="candidates" :loading="scanning" />

  <NModal v-model:show="endpointModal" preset="card" :title="editing ? '编辑 Docker Endpoint' : '添加 Docker Endpoint'" class="endpoint-modal">
    <NForm label-placement="top">
      <div class="two-columns"><NFormItem label="名称"><NInput v-model:value="endpointForm.name" /></NFormItem><NFormItem label="类型"><NSelect v-model:value="endpointForm.endpoint_type" :options="[{ label: '本机 Unix Socket', value: 'local_socket' }, { label: '远程 TCP API', value: 'remote_tcp' }]" /></NFormItem></div>
      <NFormItem label="Endpoint 地址"><NInput v-model:value="endpointForm.endpoint_url" :placeholder="endpointForm.endpoint_type === 'local_socket' ? 'unix:///var/run/docker.sock' : 'tcp://10.0.0.251:2376'" /></NFormItem>
      <div class="two-columns"><NFormItem label="局域网主机（可选）"><NInput v-model:value="endpointForm.lan_host" placeholder="例如 NAS 地址 10.0.0.251" /><small class="field-help">与容器发布端口组合为候选内网地址，不是 Docker API 地址。</small></NFormItem><NFormItem label="外网主机（可选）"><NInput v-model:value="endpointForm.public_host_hint" placeholder="例如 service.example.com" /><small class="field-help">仅用于生成候选外网地址，不会自动配置域名或反向代理。</small></NFormItem></div>
      <NFormItem v-if="endpointForm.endpoint_type === 'remote_tcp'" label="TLS"><NSwitch v-model:value="endpointForm.tls_enabled" /></NFormItem>
      <template v-if="endpointForm.endpoint_type === 'remote_tcp' && endpointForm.tls_enabled"><NFormItem label="TLS CA（PEM，留空保留）"><NInput v-model:value="endpointForm.tls_ca" type="textarea" /></NFormItem><NFormItem label="TLS Cert（PEM，留空保留）"><NInput v-model:value="endpointForm.tls_cert" type="textarea" /></NFormItem><NFormItem label="TLS Key（PEM，留空保留）"><NInput v-model:value="endpointForm.tls_key" type="textarea" /></NFormItem></template>
      <NButton type="primary" block @click="saveEndpoint">保存 Endpoint</NButton>
    </NForm>
  </NModal>

</template>

<style scoped>
.page-header { display: flex; align-items: end; justify-content: space-between; gap: 2rem; margin-bottom: 1.5rem; }
.page-header p { margin: 0; color: #5da9ff; font-family: "IBM Plex Mono", monospace; font-size: 0.68rem; letter-spacing: 0.2em; }
.page-header h1 { margin: 0.35rem 0; font-size: 2.35rem; }
.page-header span, .endpoint-title small { color: #75859b; }
.risk { margin-bottom: 1rem; }
.endpoint-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(20rem, 1fr)); gap: 1rem; margin-bottom: 1.2rem; }
.endpoint-title { display: flex; align-items: center; gap: 0.8rem; margin-bottom: 1rem; font-size: 1.35rem; }
.endpoint-title div { display: grid; flex: 1; min-width: 0; font-size: 0.9rem; }
.endpoint-title small { overflow: hidden; font-family: "IBM Plex Mono", monospace; font-size: 0.65rem; text-overflow: ellipsis; }
.endpoint-modal { width: min(48rem, calc(100vw - 2rem)); }
.two-columns { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; }
.candidate-tip { margin-bottom: 0.8rem; }
.candidate-tip a { color: #5da9ff; }
.field-help { display: block; margin-top: 0.35rem; color: #6f8098; line-height: 1.45; }
@media (max-width: 760px) { .page-header { align-items: flex-start; flex-direction: column; } .two-columns { grid-template-columns: 1fr; } }
</style>
