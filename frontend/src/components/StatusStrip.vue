<script setup lang="ts">
import { NPopover } from 'naive-ui'
import { computed } from 'vue'
import type { Status, StatusPoint } from '../types'

const props = defineProps<{
  points?: StatusPoint[]
  statuses?: Status[]
  title?: string
}>()

const values = computed<StatusPoint[]>(() => {
  if (props.points?.length) return props.points
  if (props.statuses?.length) return props.statuses.map((status) => ({ status }))
  return [{ status: 'unknown' }]
})

function statusText(status: Status) {
  if (status === 'up') return '正常'
  if (status === 'down') return '故障'
  if (status === 'warning') return '警告'
  return '未知'
}

function formatTime(value?: string | null) {
  if (!value) return '等待首次检查'
  return new Date(value).toLocaleString()
}

function detail(point: StatusPoint) {
  if (point.message) return point.message
  if (point.status === 'up') return 'healthy'
  if (point.status === 'warning') return 'warning'
  if (point.status === 'down') return 'down'
  return 'unknown'
}
</script>

<template>
  <div class="status-strip" :aria-label="title || '最近检查状态'">
    <NPopover v-for="(point, index) in values" :key="index" trigger="hover" placement="top" :show-arrow="true">
      <template #trigger>
        <i :class="point.status" />
      </template>
      <div class="status-popover">
        <strong :class="point.status">{{ statusText(point.status) }}</strong>
        <span>{{ formatTime(point.checked_at) }}</span>
        <small v-if="point.latency_ms != null">响应时间：{{ point.latency_ms }} ms</small>
        <small v-if="point.status_code != null">状态码：{{ point.status_code }}</small>
        <p>{{ detail(point) }}</p>
      </div>
    </NPopover>
  </div>
</template>

<style scoped>
.status-strip { display: flex; width: 100%; height: 0.64rem; gap: 2px; }
.status-strip i { display: block; min-width: 3px; flex: 1; height: 100%; border-radius: 999px; background: #334155; cursor: help; }
.status-strip i.up { background: #4ade80; }
.status-strip i.down { background: #f43f5e; }
.status-strip i.warning { background: #f59e0b; }
.status-popover { display: grid; min-width: 11rem; gap: 0.25rem; color: var(--sc-text); }
.status-popover strong { font-size: 0.95rem; }
.status-popover strong.up { color: #4ade80; }
.status-popover strong.down { color: #fb7185; }
.status-popover strong.warning { color: #fbbf24; }
.status-popover span { color: var(--sc-muted); font-family: "IBM Plex Mono", monospace; }
.status-popover small { color: var(--sc-subtle); }
.status-popover p { margin: 0.25rem 0 0; padding-top: 0.35rem; border-top: 1px solid rgb(148 163 184 / 20%); color: var(--sc-text); }
</style>
