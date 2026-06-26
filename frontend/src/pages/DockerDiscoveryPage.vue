<script setup lang="ts">
import { BrandDocker, ChartRadar, Edit, PlugConnected, Plus, Trash } from '@vicons/tabler'
import { NAlert, NButton, NCard, NIcon, NInput, NSpace, useDialog, useMessage } from 'naive-ui'
import { computed, onMounted, ref } from 'vue'
import { RouterLink } from 'vue-router'
import { dockerApi } from '../api/docker'
import DockerCandidateTable from '../components/DockerCandidateTable.vue'
import DockerEndpointModal from '../components/DockerEndpointModal.vue'
import type { DockerCandidate, DockerEndpoint } from '../types'

const endpoints = ref<DockerEndpoint[]>([])
const candidates = ref<DockerCandidate[]>([])
const editing = ref<DockerEndpoint | null>(null)
const endpointModal = ref(false)
const scanning = ref(false)
const search = ref('')
const message = useMessage()
const dialog = useDialog()
const filteredEndpoints = computed(() => {
  const keyword = search.value.trim().toLowerCase()
  if (!keyword) return endpoints.value
  return endpoints.value.filter((endpoint) =>
    searchableText(
      endpoint.name,
      endpoint.endpoint_type,
      endpoint.endpoint_url,
      endpoint.lan_host,
      endpoint.public_host_hint,
      endpointTypeText(endpoint),
    ).includes(keyword),
  )
})

async function load() {
  endpoints.value = await dockerApi.endpoints()
}

function openEndpoint(endpoint?: DockerEndpoint) {
  editing.value = endpoint ?? null
  endpointModal.value = true
}

async function endpointSaved() {
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
    maskClosable: false,
    onPositiveClick: async () => {
      await dockerApi.removeEndpoint(endpoint.id)
      await load()
    },
  })
}

function endpointTypeText(endpoint: DockerEndpoint) {
  if (endpoint.tls_enabled) return 'TLS'
  if (endpoint.endpoint_type === 'local_socket') return '本机 Socket'
  return '远程 TCP'
}

function searchableText(...values: Array<string | null | undefined>) {
  return values.filter(Boolean).join(' ').toLowerCase()
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
  <NInput v-model:value="search" clearable placeholder="搜索 Endpoint 名称、地址或类型" class="endpoint-search" />
  <section class="endpoint-grid">
    <NCard v-for="endpoint in filteredEndpoints" :key="endpoint.id" size="small" class="endpoint-card">
      <div class="endpoint-title">
        <NIcon :component="BrandDocker" />
        <div><strong>{{ endpoint.name }}</strong><small>{{ endpoint.endpoint_url }}</small></div>
        <span class="endpoint-kind" :class="{ secure: endpoint.tls_enabled }">{{ endpointTypeText(endpoint) }}</span>
      </div>
      <NSpace size="small" class="endpoint-actions">
        <NButton size="small" secondary type="success" @click="testEndpoint(endpoint)"><template #icon><NIcon :component="PlugConnected" /></template>测试</NButton>
        <NButton size="small" type="primary" :loading="scanning" @click="scan(endpoint)"><template #icon><NIcon :component="ChartRadar" /></template>扫描</NButton>
        <NButton size="small" secondary type="info" @click="openEndpoint(endpoint)"><NIcon :component="Edit" /></NButton>
        <NButton size="small" secondary type="error" @click="removeEndpoint(endpoint)"><NIcon :component="Trash" /></NButton>
      </NSpace>
    </NCard>
  </section>
  <NAlert v-if="candidates.length" type="info" :bordered="false" class="candidate-tip">
    已生成候选列表。<RouterLink to="/admin/services">前往服务页面添加并关联</RouterLink>
  </NAlert>
  <DockerCandidateTable :candidates="candidates" :loading="scanning" />

  <DockerEndpointModal v-model:show="endpointModal" :endpoint="editing" @saved="endpointSaved" />

</template>

<style scoped>
.page-header { display: flex; align-items: end; justify-content: space-between; gap: 2rem; margin-bottom: 1.5rem; }
.page-header p { margin: 0; color: #5da9ff; font-family: "IBM Plex Mono", monospace; font-size: 0.68rem; letter-spacing: 0.2em; }
.page-header h1 { margin: 0.35rem 0; font-size: 2.35rem; }
.page-header span, .endpoint-title small { color: #75859b; }
.risk { margin-bottom: 1rem; }
.endpoint-search { max-width: 28rem; margin-bottom: 0.8rem; }
.endpoint-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(22rem, 1fr)); gap: 0.85rem; margin-bottom: 1rem; }
.endpoint-card { min-height: 8.2rem; }
.endpoint-card :deep(.n-card__content) { display: grid; min-height: 8.2rem; align-content: space-between; padding: 0.9rem; }
.endpoint-title { display: flex; align-items: flex-start; gap: 0.65rem; margin-bottom: 0.8rem; min-width: 0; font-size: 1rem; }
.endpoint-title div { display: grid; flex: 1; min-width: 0; font-size: 0.9rem; }
.endpoint-title strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.endpoint-title small { overflow-wrap: anywhere; color: var(--sc-muted); font-family: "IBM Plex Mono", monospace; font-size: 0.68rem; line-height: 1.45; }
.endpoint-kind { flex: 0 0 auto; padding: 0.2rem 0.5rem; border: 1px solid rgb(148 163 184 / 18%); border-radius: 999px; background: rgb(148 163 184 / 8%); color: var(--sc-muted); font-size: 0.66rem; line-height: 1; }
.endpoint-kind.secure { border-color: rgb(52 211 153 / 24%); background: rgb(52 211 153 / 10%); color: var(--sc-success); }
.endpoint-actions { flex-wrap: wrap; }
.candidate-tip { margin-bottom: 0.8rem; }
.candidate-tip a { color: #5da9ff; }
@media (max-width: 760px) { .page-header { align-items: flex-start; flex-direction: column; } .endpoint-grid { grid-template-columns: 1fr; } }
</style>
