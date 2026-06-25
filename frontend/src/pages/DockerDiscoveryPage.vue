<script setup lang="ts">
import { BrandDocker, ChartRadar, Edit, PlugConnected, Plus, Trash } from '@vicons/tabler'
import { NAlert, NButton, NCard, NIcon, NSpace, NTag, useDialog, useMessage } from 'naive-ui'
import { onMounted, ref } from 'vue'
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
const message = useMessage()
const dialog = useDialog()

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
    <NCard v-for="endpoint in endpoints" :key="endpoint.id" size="small" class="endpoint-card">
      <div class="endpoint-title">
        <NIcon :component="BrandDocker" />
        <div><strong>{{ endpoint.name }}</strong><small>{{ endpoint.endpoint_url }}</small></div>
        <NTag size="small" :type="endpoint.tls_enabled ? 'success' : 'default'">{{ endpoint.tls_enabled ? 'TLS' : endpoint.endpoint_type }}</NTag>
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
.endpoint-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(16rem, 1fr)); gap: 0.7rem; margin-bottom: 1rem; }
.endpoint-card :deep(.n-card__content) { padding: 0.75rem; }
.endpoint-title { display: flex; align-items: center; gap: 0.55rem; margin-bottom: 0.65rem; min-width: 0; font-size: 1rem; }
.endpoint-title div { display: grid; flex: 1; min-width: 0; font-size: 0.9rem; }
.endpoint-title strong, .endpoint-title small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.endpoint-title small { font-family: "IBM Plex Mono", monospace; font-size: 0.65rem; }
.endpoint-actions { flex-wrap: nowrap; }
.candidate-tip { margin-bottom: 0.8rem; }
.candidate-tip a { color: #5da9ff; }
@media (max-width: 760px) { .page-header { align-items: flex-start; flex-direction: column; } }
</style>
