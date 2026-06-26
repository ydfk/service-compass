<script setup lang="ts">
import { NDataTable, NInput, NTag, type DataTableColumns, type PaginationProps } from 'naive-ui'
import { computed, h, reactive, ref } from 'vue'
import type { DockerCandidate } from '../types'

const props = defineProps<{ candidates: DockerCandidate[]; loading?: boolean }>()
const search = ref('')
const filteredCandidates = computed(() => {
  const keyword = search.value.trim().toLowerCase()
  if (!keyword) return props.candidates
  return props.candidates.filter((candidate) =>
    searchableText(
      candidate.suggested_name,
      candidate.container_name,
      candidate.image,
      candidate.state,
      candidate.status,
      candidate.compose_project,
      candidate.compose_service,
      candidate.local_url,
      candidate.public_url,
      candidate.ports.join(' '),
    ).includes(keyword),
  )
})

const columns: DataTableColumns<DockerCandidate> = [
  { title: '建议名称', key: 'suggested_name', render: (row) => h('strong', row.suggested_name) },
  { title: '容器', key: 'container_name' },
  { title: '镜像', key: 'image', ellipsis: { tooltip: true } },
  {
    title: '状态',
    key: 'state',
    render: (row) =>
      h(
        NTag,
        { type: row.state === 'running' ? 'success' : 'default', size: 'small', bordered: false },
        { default: () => row.state || 'unknown' },
      ),
  },
  { title: 'Compose', key: 'compose_project', render: (row) => row.compose_project || '—' },
  { title: '端口', key: 'ports', render: (row) => row.ports.join(', ') || '—' },
  { title: '建议地址', key: 'local_url', ellipsis: { tooltip: true } },
]
const pagination = reactive<PaginationProps>({
  pageSize: 20,
  pageSizes: [20, 50, 100],
  showSizePicker: true,
})

function searchableText(...values: Array<string | null | undefined>) {
  return values.filter(Boolean).join(' ').toLowerCase()
}
</script>

<template>
  <div class="candidate-table">
    <NInput v-model:value="search" clearable placeholder="搜索容器、镜像、状态、Compose 或地址" />
    <NDataTable
      :columns="columns"
      :data="filteredCandidates"
      :loading="loading"
      size="small"
      :row-key="(row: DockerCandidate) => `${row.endpoint_id}:${row.container_id}`"
      :pagination="pagination"
    />
  </div>
</template>

<style scoped>
.candidate-table {
  display: grid;
  gap: 0.75rem;
}
</style>
