<script setup lang="ts">
import { ArrowsSort, LayoutGrid, ListDetails, Login, Settings } from '@vicons/tabler'
import { NButton, NButtonGroup, NEmpty, NIcon, NSpin, useMessage } from 'naive-ui'
import { storeToRefs } from 'pinia'
import { computed, onMounted, ref } from 'vue'
import { RouterLink } from 'vue-router'
import { groupsApi } from '../api/groups'
import { monitorsApi } from '../api/monitors'
import { servicesApi } from '../api/services'
import GroupSection from '../components/GroupSection.vue'
import ServiceEditorModal from '../components/ServiceEditorModal.vue'
import UrlSwitcher from '../components/UrlSwitcher.vue'
import { useAuthStore } from '../stores/auth'
import { useDashboardStore } from '../stores/dashboard'
import type {
  DashboardGroup,
  Group,
  Monitor,
  MonitorInput,
  Service,
  ServiceInput,
  UrlMode,
} from '../types'
import {
  emptyHttpMonitor,
  emptyService,
  monitorToInput,
  serviceHttpMonitor,
  serviceToInput,
} from '../utils/serviceForms'

const dashboard = useDashboardStore()
const auth = useAuthStore()
const { groups, loading } = storeToRefs(dashboard)
const mode = ref<UrlMode>((localStorage.getItem('service-compass-url-mode') as UrlMode) || 'public')
const cardMode = ref<'compact' | 'detail'>(
  (localStorage.getItem('service-compass-card-mode') as 'compact' | 'detail') || 'compact',
)
const sorting = ref(false)
const editorShow = ref(false)
const editorGroups = ref<Group[]>([])
const editorMonitors = ref<Monitor[]>([])
const editingService = ref<Service | null>(null)
const serviceForm = ref<ServiceInput>(emptyService())
const httpMonitor = ref<MonitorInput>(emptyHttpMonitor())
const message = useMessage()
const total = computed(() =>
  groups.value.reduce((count, group) => count + group.services.length, 0),
)
const online = computed(
  () =>
    groups.value.flatMap((group) => group.services).filter((service) => service.status === 'up')
      .length,
)

function setMode(value: UrlMode) {
  mode.value = value
  localStorage.setItem('service-compass-url-mode', value)
}

function setCardMode(value: 'compact' | 'detail') {
  cardMode.value = value
  localStorage.setItem('service-compass-card-mode', value)
}

async function openEditor(service: Service) {
  ;[editorGroups.value, editorMonitors.value] = await Promise.all([
    groupsApi.list(),
    monitorsApi.list(),
  ])
  editingService.value = service
  const monitor = serviceHttpMonitor(editorMonitors.value, service.id)
  serviceForm.value = serviceToInput(service, monitor)
  httpMonitor.value = monitor ? monitorToInput(monitor) : emptyHttpMonitor()
  editorShow.value = true
}

async function saveService() {
  const service = editingService.value
  if (!service || !serviceForm.value.name.trim()) return
  if (!serviceForm.value.local_url && !serviceForm.value.public_url) {
    return message.warning('至少填写一个访问地址')
  }
  await servicesApi.update(service.id, {
    ...serviceForm.value,
    monitor: serviceForm.value.create_monitor ? httpMonitor.value : null,
  })
  editorShow.value = false
  message.success('服务已更新')
  await dashboard.load()
}

async function moveService(group: DashboardGroup, service: Service, direction: -1 | 1) {
  const current = group.services.findIndex((item) => item.id === service.id)
  const target = current + direction
  if (current < 0 || target < 0 || target >= group.services.length) return
  const ordered = [...group.services]
  ;[ordered[current], ordered[target]] = [ordered[target], ordered[current]]
  await servicesApi.reorder(ordered.map((item, index) => ({ id: item.id, sort_order: index })))
  await dashboard.load()
}

onMounted(async () => {
  await Promise.all([dashboard.load(), auth.verify()])
})
</script>

<template>
  <div class="dashboard-shell">
    <header class="topbar">
      <RouterLink class="brand" to="/"><img src="../assets/logo.svg" alt="" /><span><strong>ServiceCompass</strong><small>{{ total }} 个服务 · {{ online }} 在线</small></span></RouterLink>
      <div class="header-actions">
        <UrlSwitcher :mode="mode" @change="setMode" />
        <NButtonGroup>
          <NButton size="small" :type="cardMode === 'compact' ? 'primary' : 'default'" title="小卡片" @click="setCardMode('compact')"><template #icon><NIcon :component="LayoutGrid" /></template>小卡</NButton>
          <NButton size="small" :type="cardMode === 'detail' ? 'primary' : 'default'" title="监控详情卡片" @click="setCardMode('detail')"><template #icon><NIcon :component="ListDetails" /></template>详情</NButton>
        </NButtonGroup>
        <NButton v-if="auth.authenticated" size="small" :type="sorting ? 'warning' : 'default'" @click="sorting = !sorting"><template #icon><NIcon :component="ArrowsSort" /></template>{{ sorting ? '完成排序' : '卡片排序' }}</NButton>
        <RouterLink :to="auth.authenticated ? '/admin' : '/login'"><NButton size="small" :type="auth.authenticated ? 'default' : 'primary'"><template #icon><NIcon :component="auth.authenticated ? Settings : Login" /></template>{{ auth.authenticated ? '管理' : '管理员登录' }}</NButton></RouterLink>
      </div>
    </header>
    <main>
      <NSpin :show="loading">
        <NEmpty v-if="!loading && total === 0" description="还没有服务，登录管理端添加第一个服务" />
        <GroupSection
          v-for="group in groups.filter((item) => item.services.length)"
          :key="group.id"
          :group="group"
          :mode="mode"
          :card-mode="cardMode"
          :editable="auth.authenticated"
          :sorting="sorting"
          @edit="openEditor"
          @move="moveService"
        />
      </NSpin>
    </main>
    <ServiceEditorModal
      v-model:show="editorShow"
      v-model:form="serviceForm"
      v-model:monitor="httpMonitor"
      :groups="editorGroups"
      :editing="true"
      @save="saveService"
    />
  </div>
</template>

<style scoped>
.dashboard-shell { width: min(88rem, 100%); margin: auto; padding: 0 1.5rem 4rem; }
.topbar { display: flex; min-height: 4.6rem; align-items: center; justify-content: space-between; gap: 1rem; border-bottom: 1px solid rgb(148 163 184 / 10%); }
.brand { display: flex; flex: 0 0 auto; align-items: center; gap: 0.7rem; color: inherit; text-decoration: none; }
.brand img { width: 2.2rem; }.brand span { display: grid; }.brand strong { font-family: "IBM Plex Mono", monospace; font-size: 0.86rem; }.brand small { color: #66768d; font-size: 0.66rem; }
.header-actions { display: flex; flex-wrap: wrap; align-items: center; justify-content: flex-end; gap: 0.55rem; }
.header-actions a { text-decoration: none; }
main { padding-top: 0.4rem; }
@media (max-width: 760px) { .dashboard-shell { padding-inline: 0.8rem; } .topbar { align-items: flex-start; flex-direction: column; padding: 0.9rem 0; } .header-actions { width: 100%; justify-content: flex-start; } }
</style>
