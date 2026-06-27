<script setup lang="ts">
import { ArrowsSort, LayoutGrid, ListDetails, Login, Settings } from '@vicons/tabler'
import { NButton, NButtonGroup, NEmpty, NIcon, NSpin, useMessage } from 'naive-ui'
import { storeToRefs } from 'pinia'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { RouterLink } from 'vue-router'
import { api } from '../api/client'
import { groupsApi } from '../api/groups'
import { monitorsApi } from '../api/monitors'
import { servicesApi } from '../api/services'
import GroupSection from '../components/GroupSection.vue'
import ServiceEditorModal from '../components/ServiceEditorModal.vue'
import ThemeToggle from '../components/ThemeToggle.vue'
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
  Space,
  UrlMode,
} from '../types'
import {
  cleanServiceInput,
  emptyHttpMonitor,
  emptyService,
  monitorToInput,
  serviceCertMonitor,
  serviceHttpMonitor,
  serviceToInput,
  UNGROUPED_ID,
} from '../utils/serviceForms'

const dashboard = useDashboardStore()
const auth = useAuthStore()
const { spaces, groups, loading } = storeToRefs(dashboard)
const mode = ref<UrlMode>((localStorage.getItem('service-compass-url-mode') as UrlMode) || 'public')
const cardMode = ref<'compact' | 'detail'>(
  (localStorage.getItem('service-compass-card-mode') as 'compact' | 'detail') || 'detail',
)
const sorting = ref(false)
const draggingGroupId = ref<string | null>(null)
const editorShow = ref(false)
const editorTitle = ref('添加服务')
const editorGroups = ref<Group[]>([])
const editorSpaces = ref<Space[]>([])
const editorMonitors = ref<Monitor[]>([])
const editingService = ref<Service | null>(null)
const serviceForm = ref<ServiceInput>(emptyService())
const httpMonitor = ref<MonitorInput>(emptyHttpMonitor())
const appVersion = ref('')
const message = useMessage()
const total = computed(() =>
  groups.value.reduce((count, group) => count + group.services.length, 0),
)
const online = computed(
  () =>
    groups.value.flatMap((group) => group.services).filter((service) => service.status === 'up')
      .length,
)
const visibleGroups = computed(() =>
  groups.value.filter((group) => auth.authenticated || group.services.length),
)
const visibleSpaces = computed(() =>
  spaces.value
    .map((space) => ({
      ...space,
      groups: space.groups.filter((group) => auth.authenticated || group.services.length),
    }))
    .filter((space) => auth.authenticated || space.groups.length),
)
const activeSpaceId = ref('')
const activeSpace = computed(
  () =>
    visibleSpaces.value.find((space) => space.id === activeSpaceId.value) ?? visibleSpaces.value[0],
)
const activeGroups = computed(() => activeSpace.value?.groups ?? visibleGroups.value)

watch(visibleSpaces, (value) => {
  if (!value.length) {
    activeSpaceId.value = ''
    return
  }
  if (!value.some((space) => space.id === activeSpaceId.value)) activeSpaceId.value = value[0].id
})

function setMode(value: UrlMode) {
  mode.value = value
  localStorage.setItem('service-compass-url-mode', value)
}

function setCardMode(value: 'compact' | 'detail') {
  cardMode.value = value
  localStorage.setItem('service-compass-card-mode', value)
}

async function loadEditorData() {
  ;[editorGroups.value, editorSpaces.value, editorMonitors.value] = await Promise.all([
    groupsApi.list(),
    groupsApi.spaces(),
    monitorsApi.list(),
  ])
}

async function openEditor(service: Service) {
  await loadEditorData()
  editingService.value = service
  editorTitle.value = '编辑服务'
  const monitor = serviceHttpMonitor(editorMonitors.value, service.id)
  const certMonitor = serviceCertMonitor(editorMonitors.value, service.id)
  serviceForm.value = serviceToInput(service, monitor, certMonitor)
  httpMonitor.value = monitor ? monitorToInput(monitor) : emptyHttpMonitor()
  editorShow.value = true
}

async function openCreate(group: DashboardGroup) {
  await loadEditorData()
  editingService.value = null
  editorTitle.value = '添加服务'
  serviceForm.value = emptyService()
  serviceForm.value.group_id = group.id === UNGROUPED_ID ? '' : group.id
  serviceForm.value.sort_order = group.services.length
  httpMonitor.value = emptyHttpMonitor()
  editorShow.value = true
}

async function openClone(service: Service) {
  await loadEditorData()
  editingService.value = null
  editorTitle.value = `克隆 ${service.name}`
  const monitor = serviceHttpMonitor(editorMonitors.value, service.id)
  const certMonitor = serviceCertMonitor(editorMonitors.value, service.id)
  serviceForm.value = serviceToInput(service, monitor, certMonitor)
  serviceForm.value.name = `${service.name} 副本`
  serviceForm.value.sort_order = service.sort_order + 1
  httpMonitor.value = monitor ? monitorToInput(monitor) : emptyHttpMonitor()
  editorShow.value = true
}

async function saveService() {
  if (!serviceForm.value.name.trim()) return
  const input = {
    ...cleanServiceInput(serviceForm.value),
    monitor: serviceForm.value.create_monitor ? httpMonitor.value : null,
  }
  if (editingService.value) await servicesApi.update(editingService.value.id, input)
  else await servicesApi.create(input)
  editorShow.value = false
  message.success(editingService.value ? '服务已更新' : '服务已添加')
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

function startGroupDrag(group: DashboardGroup) {
  if (!sorting.value || group.id === UNGROUPED_ID) return
  draggingGroupId.value = group.id
}

async function dropGroup(target: DashboardGroup) {
  const sourceId = draggingGroupId.value
  draggingGroupId.value = null
  if (!sourceId || sourceId === target.id || target.id === UNGROUPED_ID) return
  const ordered = activeGroups.value.filter((group) => group.id !== UNGROUPED_ID)
  const sourceIndex = ordered.findIndex((group) => group.id === sourceId)
  const targetIndex = ordered.findIndex((group) => group.id === target.id)
  if (sourceIndex < 0 || targetIndex < 0) return
  const [source] = ordered.splice(sourceIndex, 1)
  ordered.splice(targetIndex, 0, source)
  await groupsApi.reorder(ordered.map((group, index) => ({ id: group.id, sort_order: index })))
  await dashboard.load()
}

function addEditorGroup(group: Group) {
  if (!editorGroups.value.some((item) => item.id === group.id)) editorGroups.value.push(group)
}

function spaceTotal(space: { groups: DashboardGroup[] }) {
  return space.groups.reduce((count, group) => count + group.services.length, 0)
}

function spaceOnline(space: { groups: DashboardGroup[] }) {
  return space.groups.reduce(
    (count, group) => count + group.services.filter((service) => service.status === 'up').length,
    0,
  )
}

async function loadVersion() {
  try {
    const health = await api<{ version: string }>('/api/health')
    appVersion.value = health.version
  } catch {
    appVersion.value = ''
  }
}

onMounted(async () => {
  await Promise.all([dashboard.load(), auth.verify(), loadVersion()])
  dashboard.startAutoRefresh()
})

onUnmounted(() => {
  dashboard.stopAutoRefresh()
})
</script>

<template>
  <div class="dashboard-shell">
    <header class="topbar">
      <RouterLink class="brand" to="/"><img src="../assets/logo.svg" alt="" /><span><strong>ServiceCompass</strong><small>{{ total }} 个服务 · {{ online }} 在线</small></span></RouterLink>
      <div class="header-actions">
        <UrlSwitcher :mode="mode" @change="setMode" />
        <ThemeToggle />
        <NButtonGroup>
          <NButton size="small" :type="cardMode === 'compact' ? 'primary' : 'default'" title="小卡片" @click="setCardMode('compact')"><template #icon><NIcon :component="LayoutGrid" /></template>小卡</NButton>
          <NButton size="small" :type="cardMode === 'detail' ? 'primary' : 'default'" title="监控详情卡片" @click="setCardMode('detail')"><template #icon><NIcon :component="ListDetails" /></template>详情</NButton>
        </NButtonGroup>
        <NButton v-if="auth.authenticated" size="small" :type="sorting ? 'warning' : 'default'" @click="sorting = !sorting"><template #icon><NIcon :component="ArrowsSort" /></template>{{ sorting ? '完成排序' : '服务与分组排序' }}</NButton>
        <RouterLink :to="auth.authenticated ? '/admin' : '/login'"><NButton size="small" :type="auth.authenticated ? 'default' : 'primary'"><template #icon><NIcon :component="auth.authenticated ? Settings : Login" /></template>{{ auth.authenticated ? '管理' : '管理员登录' }}</NButton></RouterLink>
      </div>
    </header>
    <main>
      <NSpin :show="loading">
        <NEmpty v-if="!loading && total === 0" description="还没有服务，登录管理端添加第一个服务" />
        <nav v-if="visibleSpaces.length > 1" class="space-tabs" aria-label="空间切换">
          <button
            v-for="space in visibleSpaces"
            :key="space.id"
            type="button"
            :class="{ active: space.id === activeSpaceId }"
            @click="activeSpaceId = space.id"
          >
            <span>{{ space.name }}</span>
            <small>{{ spaceOnline(space) }} / {{ spaceTotal(space) }}</small>
          </button>
        </nav>
        <div
          v-for="group in activeGroups"
          :key="group.id"
          class="group-wrapper"
          :class="{ draggable: sorting && group.id !== UNGROUPED_ID, dragging: draggingGroupId === group.id }"
          :draggable="sorting && group.id !== UNGROUPED_ID"
          @dragstart="startGroupDrag(group)"
          @dragend="draggingGroupId = null"
          @dragover.prevent
          @drop.prevent="dropGroup(group)"
        >
          <GroupSection
            :group="group"
            :mode="mode"
            :card-mode="cardMode"
            :editable="auth.authenticated"
            :sorting="sorting"
            @add="openCreate"
            @clone="openClone"
            @edit="openEditor"
            @move="moveService"
          />
        </div>
      </NSpin>
    </main>
    <ServiceEditorModal
      v-model:show="editorShow"
      v-model:form="serviceForm"
      v-model:monitor="httpMonitor"
      :groups="editorGroups"
      :spaces="editorSpaces"
      :editing="Boolean(editingService)"
      :title="editorTitle"
      @group-created="addEditorGroup"
      @save="saveService"
    />
    <footer class="public-footer">ServiceCompass · {{ appVersion || '0.1.0' }} · © 2026</footer>
  </div>
</template>

<style scoped>
.dashboard-shell { width: min(108rem, 100%); margin: auto; padding: 0 1.5rem 4rem; }
.topbar { display: flex; min-height: 4.6rem; align-items: center; justify-content: space-between; gap: 1rem; border-bottom: 1px solid rgb(148 163 184 / 10%); }
.brand { display: flex; flex: 0 0 auto; align-items: center; gap: 0.7rem; color: inherit; text-decoration: none; }
.brand img { width: 2.2rem; }.brand span { display: grid; }.brand strong { font-family: "IBM Plex Mono", monospace; font-size: 0.86rem; }.brand small { color: var(--sc-muted); font-size: 0.66rem; }
.header-actions { display: flex; flex-wrap: wrap; align-items: center; justify-content: flex-end; gap: 0.55rem; }
.header-actions a { text-decoration: none; }
main { padding-top: 0.4rem; }
.space-tabs { display: inline-flex; max-width: 100%; gap: 0.35rem; margin: 1rem 0 0.4rem; padding: 0.28rem; overflow-x: auto; border: 1px solid rgb(148 163 184 / 12%); border-radius: 999px; background: rgb(15 23 42 / 36%); backdrop-filter: blur(12px); }
.space-tabs button { display: inline-flex; flex: 0 0 auto; align-items: center; gap: 0.45rem; padding: 0.42rem 0.75rem; border: 0; border-radius: 999px; background: transparent; color: var(--sc-muted); cursor: pointer; font-size: 0.82rem; transition: background 160ms ease, color 160ms ease; }
.space-tabs button:hover { color: var(--sc-text); }
.space-tabs button.active { background: rgb(96 165 250 / 18%); color: #bfdbfe; box-shadow: inset 0 0 0 1px rgb(147 197 253 / 18%); }
.space-tabs small { min-width: 2.6rem; padding: 0.05rem 0.38rem; border-radius: 999px; background: rgb(148 163 184 / 14%); color: var(--sc-subtle); font-family: "IBM Plex Mono", monospace; font-size: 0.65rem; text-align: center; }
.public-footer { margin-top: 3rem; color: var(--sc-subtle); font-family: "IBM Plex Mono", monospace; font-size: 0.68rem; text-align: center; }
.group-wrapper.draggable { cursor: grab; }
.group-wrapper.draggable :deep(.group-section > header) { padding-left: 0.6rem; border-left: 2px solid rgb(96 165 250 / 45%); }
.group-wrapper.dragging { opacity: 0.45; }
@media (max-width: 760px) { .dashboard-shell { padding-inline: 0.8rem; } .topbar { align-items: flex-start; flex-direction: column; padding: 0.9rem 0; } .header-actions { width: 100%; justify-content: flex-start; } }
</style>
