<script setup lang="ts">
import { NDataTable, NTag, type DataTableColumns } from 'naive-ui'
import { h } from 'vue'
import type { DockerCandidate } from '../types'

defineProps<{ candidates: DockerCandidate[]; loading?: boolean }>()

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
</script>

<template>
  <NDataTable
    :columns="columns"
    :data="candidates"
    :loading="loading"
    :row-key="(row: DockerCandidate) => `${row.endpoint_id}:${row.container_id}`"
  />
</template>
