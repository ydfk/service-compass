<script setup lang="ts">
import { Copy, Edit, Plus, Trash } from '@vicons/tabler'
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
  useDialog,
  useMessage,
  type DataTableColumns,
  type PaginationProps,
} from 'naive-ui'
import { computed, h, onMounted, reactive, ref } from 'vue'
import { useRoute } from 'vue-router'
import { groupsApi, type GroupInput, type SpaceInput } from '../api/groups'
import { monitorsApi } from '../api/monitors'
import { servicesApi } from '../api/services'
import ServiceEditorModal from '../components/ServiceEditorModal.vue'
import type { Group, Monitor, MonitorInput, Service, ServiceInput, Space } from '../types'
import {
  cleanServiceInput,
  emptyHttpMonitor,
  emptyService,
  monitorToInput,
  serviceCertMonitor,
  serviceHttpMonitor,
  serviceToInput,
} from '../utils/serviceForms'
const spaces = ref<Space[]>([])
const groups = ref<Group[]>([])
const services = ref<Service[]>([])
const monitors = ref<Monitor[]>([])
const loading = ref(false)
const groupModal = ref(false)
const spaceModal = ref(false)
const serviceModal = ref(false)
const editingSpace = ref<Space | null>(null)
const editingGroup = ref<Group | null>(null)
const editingService = ref<Service | null>(null)
const serviceEditorTitle = ref('添加服务')
const draggingGroupId = ref<string | null>(null)
const search = ref('')
const selectedSpaceId = ref('')
const selectedGroupId = ref('')
const groupForm = ref<GroupInput>({ name: '', description: '', sort_order: 0 })
const spaceForm = ref<SpaceInput>({ name: '', description: '', sort_order: 0 })
const serviceForm = ref<ServiceInput>(emptyService())
const httpMonitor = ref<MonitorInput>(emptyHttpMonitor())
const message = useMessage()
const dialog = useDialog()
const route = useRoute()
const tablePagination = reactive<PaginationProps>({
  pageSize: 20,
  pageSizes: [20, 50, 100],
  showSizePicker: true,
})
const filteredServices = computed(() => {
  const keyword = search.value.trim().toLowerCase()
  return services.value.filter((service) => {
    if (selectedGroupId.value && service.group_id !== selectedGroupId.value) return false
    if (selectedSpaceId.value && serviceSpaceId(service) !== selectedSpaceId.value) return false
    if (!keyword) return true
    const groupName = groupScope(service.group_id)
    return searchableText(
      service.name,
      service.description,
      groupName,
      service.public_url,
      service.local_url,
      service.docker_name,
      service.docker_image,
      service.docker_compose_project,
      service.docker_compose_service,
    ).includes(keyword)
  })
})
const scopedGroups = computed(() =>
  selectedSpaceId.value
    ? groups.value.filter((group) => group.space_id === selectedSpaceId.value)
    : groups.value,
)
const filterSpaceOptions = computed(() => [
  { label: '全部空间', value: '' },
  ...spaces.value.map((item) => ({ label: item.name, value: item.id })),
])
const spaceOptions = computed(() =>
  spaces.value.map((item) => ({ label: item.name, value: item.id })),
)
const groupOptions = computed(() => [
  { label: '全部分组', value: '' },
  ...scopedGroups.value.map((item) => ({
    label: `${spaceName(item.space_id)} / ${item.name}`,
    value: item.id,
  })),
])

const columns: DataTableColumns<Service> = [
  { title: '服务', key: 'name', render: (row) => h('strong', row.name) },
  {
    title: '空间 / 分组',
    key: 'group_id',
    render: (row) => groupScope(row.group_id),
  },
  {
    title: '外网地址',
    key: 'public_url',
    ellipsis: { tooltip: true },
    render: (row) => row.public_url || '—',
  },
  {
    title: '内网地址',
    key: 'local_url',
    ellipsis: { tooltip: true },
    render: (row) => row.local_url || '—',
  },
  {
    title: '监控',
    key: 'monitor',
    render: (row) =>
      monitors.value.some((item) => item.service_id === row.id && item.enabled)
        ? '已启用'
        : '未启用',
  },
  {
    title: '操作',
    key: 'actions',
    render: (row) =>
      h(NSpace, null, {
        default: () => [
          action(Edit, '编辑', 'info', () => openService(row)),
          action(Copy, '克隆', 'warning', () => cloneService(row)),
          action(Trash, '删除', 'error', () => removeService(row)),
        ],
      }),
  },
]

function action(
  icon: typeof Edit,
  label: string,
  type: 'info' | 'warning' | 'error',
  onClick: () => void,
) {
  return h(
    NButton,
    { size: 'small', secondary: true, type, onClick },
    { icon: () => h(NIcon, { component: icon }), default: () => label },
  )
}

function searchableText(...values: Array<string | null | undefined>) {
  return values.filter(Boolean).join(' ').toLowerCase()
}

function spaceName(id?: string | null) {
  return spaces.value.find((item) => item.id === id)?.name ?? '默认空间'
}

function groupScope(groupId: string) {
  const group = groups.value.find((item) => item.id === groupId)
  if (!group) return '默认空间 / 未分组'
  return `${spaceName(group.space_id)} / ${group.name}`
}

function serviceSpaceId(service: Service) {
  return (
    groups.value.find((group) => group.id === service.group_id)?.space_id || spaces.value[0]?.id
  )
}

function groupsInSpace(spaceId: string) {
  return groups.value.filter((group) => group.space_id === spaceId)
}

function serviceCount(groupId: string) {
  return services.value.filter((service) => service.group_id === groupId).length
}

function spaceServiceCount(spaceId: string) {
  return groupsInSpace(spaceId).reduce((count, group) => count + serviceCount(group.id), 0)
}

function selectSpace(spaceId: string) {
  selectedSpaceId.value = selectedSpaceId.value === spaceId ? '' : spaceId
  selectedGroupId.value = ''
}

function selectGroup(groupId: string, spaceId: string) {
  selectedSpaceId.value = spaceId
  selectedGroupId.value = selectedGroupId.value === groupId ? '' : groupId
}

async function load() {
  loading.value = true
  try {
    ;[spaces.value, groups.value, services.value, monitors.value] = await Promise.all([
      groupsApi.spaces(),
      groupsApi.list(),
      servicesApi.list(),
      monitorsApi.list(),
    ])
  } finally {
    loading.value = false
  }
}

function openSpace(space?: Space) {
  editingSpace.value = space ?? null
  spaceForm.value = space
    ? {
        name: space.name,
        description: space.description,
        sort_order: space.sort_order,
      }
    : { name: '', description: '', sort_order: spaces.value.length }
  spaceModal.value = true
}

async function saveSpace() {
  if (!spaceForm.value.name.trim()) return message.warning('请输入空间名称')
  if (editingSpace.value) await groupsApi.updateSpace(editingSpace.value.id, spaceForm.value)
  else await groupsApi.createSpace(spaceForm.value)
  spaceModal.value = false
  message.success('空间已保存')
  await load()
}

function openGroup(group?: Group) {
  editingGroup.value = group ?? null
  groupForm.value = group
    ? {
        space_id: group.space_id,
        name: group.name,
        description: group.description,
        icon: group.icon,
        sort_order: group.sort_order,
      }
    : {
        space_id: spaces.value[0]?.id ?? null,
        name: '',
        description: '',
        sort_order: groups.value.length,
      }
  groupModal.value = true
}

async function saveGroup() {
  if (!groupForm.value.name.trim()) return message.warning('请输入分组名称')
  if (editingGroup.value) await groupsApi.update(editingGroup.value.id, groupForm.value)
  else await groupsApi.create(groupForm.value)
  groupModal.value = false
  message.success('分组已保存')
  await load()
}

function openService(service?: Service) {
  editingService.value = service ?? null
  serviceEditorTitle.value = service ? '编辑服务' : '添加服务'
  const monitor = service ? serviceHttpMonitor(monitors.value, service.id) : undefined
  const certMonitor = service ? serviceCertMonitor(monitors.value, service.id) : undefined
  serviceForm.value = service ? serviceToInput(service, monitor, certMonitor) : emptyService()
  httpMonitor.value = monitor ? monitorToInput(monitor) : emptyHttpMonitor()
  serviceModal.value = true
}

function cloneService(service: Service) {
  editingService.value = null
  serviceEditorTitle.value = `克隆 ${service.name}`
  const monitor = serviceHttpMonitor(monitors.value, service.id)
  const certMonitor = serviceCertMonitor(monitors.value, service.id)
  serviceForm.value = serviceToInput(service, monitor, certMonitor)
  serviceForm.value.name = `${service.name} 副本`
  serviceForm.value.sort_order = service.sort_order + 1
  httpMonitor.value = monitor ? monitorToInput(monitor) : emptyHttpMonitor()
  serviceModal.value = true
}

async function saveService() {
  if (!serviceForm.value.name.trim()) return message.warning('请填写服务名称')
  const input = {
    ...cleanServiceInput(serviceForm.value),
    monitor: serviceForm.value.create_monitor ? httpMonitor.value : null,
  }
  if (editingService.value) await servicesApi.update(editingService.value.id, input)
  else await servicesApi.create(input)
  serviceModal.value = false
  message.success('服务、Docker 关联与监控设置已保存')
  await load()
}

function startGroupDrag(group: Group) {
  draggingGroupId.value = group.id
}

async function dropGroup(target: Group) {
  const sourceId = draggingGroupId.value
  draggingGroupId.value = null
  if (!sourceId || sourceId === target.id) return
  const ordered = [...groups.value]
  const sourceIndex = ordered.findIndex((group) => group.id === sourceId)
  const targetIndex = ordered.findIndex((group) => group.id === target.id)
  if (sourceIndex < 0 || targetIndex < 0) return
  const [source] = ordered.splice(sourceIndex, 1)
  ordered.splice(targetIndex, 0, source)
  groups.value = ordered
  await groupsApi.reorder(ordered.map((group, index) => ({ id: group.id, sort_order: index })))
  message.success('分组顺序已更新')
}

function addGroup(group: Group) {
  if (!groups.value.some((item) => item.id === group.id)) groups.value.push(group)
}

function removeService(service: Service) {
  dialog.warning({
    title: '删除服务',
    content: `确认删除 ${service.name}？`,
    positiveText: '删除',
    negativeText: '取消',
    maskClosable: false,
    onPositiveClick: async () => {
      await servicesApi.remove(service.id)
      await load()
    },
  })
}

onMounted(async () => {
  await load()
  const editId = typeof route.query.edit === 'string' ? route.query.edit : null
  if (editId) {
    const service = services.value.find((item) => item.id === editId)
    if (service) openService(service)
  }
})
</script>

<template>
  <header class="page-header"><div><p>SERVICE CATALOG</p><h1>服务</h1><span>服务是核心；Docker 关联和监控均可选。</span></div><NSpace><NButton @click="openSpace()">新建空间</NButton><NButton @click="openGroup()">新建分组</NButton><NButton type="primary" @click="openService()"><template #icon><NIcon :component="Plus" /></template>添加服务</NButton></NSpace></header>
  <section v-if="spaces.length" class="relation-board">
    <NCard
      v-for="space in spaces"
      :key="space.id"
      size="small"
      class="space-card"
      :class="{ active: selectedSpaceId === space.id && !selectedGroupId }"
    >
      <div class="space-head" @click="selectSpace(space.id)">
        <div><strong>{{ space.name }}</strong><small>{{ groupsInSpace(space.id).length }} 分组 · {{ spaceServiceCount(space.id) }} 服务</small></div>
        <NButton quaternary circle size="small" @click.stop="openSpace(space)"><NIcon :component="Edit" /></NButton>
      </div>
      <div class="group-chain">
        <button
          v-for="group in groupsInSpace(space.id)"
          :key="group.id"
          type="button"
          :class="{ active: selectedGroupId === group.id }"
          draggable="true"
          @click="selectGroup(group.id, space.id)"
          @dragstart="startGroupDrag(group)"
          @dragend="draggingGroupId = null"
          @dragover.prevent
          @drop.prevent="dropGroup(group)"
        >
          <span>{{ group.name }}</span>
          <small>{{ serviceCount(group.id) }}</small>
          <NButton quaternary circle size="tiny" @click.stop="openGroup(group)"><NIcon :component="Edit" /></NButton>
        </button>
      </div>
    </NCard>
  </section>
  <NCard class="filter-card" size="small">
    <NSpace>
      <NSelect v-model:value="selectedSpaceId" :options="filterSpaceOptions" class="filter-select" @update:value="selectedGroupId = ''" />
      <NSelect v-model:value="selectedGroupId" :options="groupOptions" class="filter-select" />
      <NInput v-model:value="search" clearable placeholder="搜索服务、分组、地址、Docker 名称或镜像" class="filter-search" />
    </NSpace>
  </NCard>
  <NDataTable
    :columns="columns"
    :data="filteredServices"
    :loading="loading"
    :row-key="(row: Service) => row.id"
    :pagination="tablePagination"
  />
  <NModal
    v-model:show="spaceModal"
    preset="card"
    :title="editingSpace ? '编辑空间' : '新建空间'"
    class="group-modal"
    :mask-closable="false"
  >
    <NForm>
      <NFormItem label="名称"><NInput v-model:value="spaceForm.name" /></NFormItem>
      <NFormItem label="说明"><NInput v-model:value="spaceForm.description" type="textarea" /></NFormItem>
      <NFormItem label="排序"><NInputNumber v-model:value="spaceForm.sort_order" /></NFormItem>
      <NButton type="primary" block @click="saveSpace">保存空间</NButton>
    </NForm>
  </NModal>
  <NModal
    v-model:show="groupModal"
    preset="card"
    :title="editingGroup ? '编辑分组' : '新建分组'"
    class="group-modal"
    :mask-closable="false"
  >
    <NForm>
      <NFormItem label="空间"><NSelect v-model:value="groupForm.space_id" :options="spaceOptions" placeholder="选择所属空间" /></NFormItem>
      <NFormItem label="名称"><NInput v-model:value="groupForm.name" /></NFormItem>
      <NFormItem label="说明"><NInput v-model:value="groupForm.description" type="textarea" /></NFormItem>
      <NFormItem label="排序"><NInputNumber v-model:value="groupForm.sort_order" /></NFormItem>
      <NButton type="primary" block @click="saveGroup">保存分组</NButton>
    </NForm>
  </NModal>
  <ServiceEditorModal v-model:show="serviceModal" v-model:form="serviceForm" v-model:monitor="httpMonitor" :groups="groups" :spaces="spaces" :editing="Boolean(editingService)" :title="serviceEditorTitle" @group-created="addGroup" @save="saveService" />
</template>

<style scoped>
.page-header { display: flex; align-items: end; justify-content: space-between; gap: 2rem; margin-bottom: 1.5rem; }
.page-header p { margin: 0; color: #5da9ff; font-family: "IBM Plex Mono", monospace; font-size: 0.68rem; letter-spacing: 0.2em; }
.page-header h1 { margin: 0.3rem 0; font-size: 2.1rem; }
.page-header span { color: #75859b; }
.relation-board { display: grid; grid-template-columns: repeat(auto-fill, minmax(18rem, 1fr)); gap: 0.75rem; margin-bottom: 1rem; }
.space-card { border-color: rgb(148 163 184 / 16%); background: color-mix(in srgb, var(--sc-card) 92%, transparent); transition: border-color 160ms ease, background 160ms ease; }
.space-card.active { border-color: rgb(96 165 250 / 42%); background: rgb(96 165 250 / 8%); }
.space-head { display: flex; align-items: center; justify-content: space-between; gap: 0.8rem; cursor: pointer; }
.space-head > div { display: grid; gap: 0.15rem; min-width: 0; }
.space-head strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.space-head small, .group-chain small { color: var(--sc-muted); }
.group-chain { display: flex; flex-wrap: wrap; gap: 0.4rem; margin-top: 0.7rem; }
.group-chain button { display: inline-flex; align-items: center; gap: 0.35rem; max-width: 100%; padding: 0.24rem 0.34rem 0.24rem 0.55rem; border: 1px solid rgb(148 163 184 / 14%); border-radius: 999px; background: rgb(148 163 184 / 7%); color: var(--sc-text); cursor: grab; font-size: 0.75rem; }
.group-chain button.active { border-color: rgb(52 211 153 / 28%); background: rgb(52 211 153 / 12%); color: var(--sc-success); }
.group-chain button > span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.filter-card { margin-bottom: 0.8rem; }
.filter-select { width: 12rem; }
.filter-search { width: min(26rem, 100%); }
.group-modal { width: min(28rem, calc(100vw - 1.5rem)); }
@media (max-width: 760px) { .page-header { align-items: flex-start; flex-direction: column; } .filter-select, .filter-search { width: 100%; } }
</style>
