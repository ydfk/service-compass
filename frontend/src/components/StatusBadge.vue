<script setup lang="ts">
import { NTag } from 'naive-ui'
import { computed } from 'vue'
import type { Status } from '../types'

const props = defineProps<{ status?: Status }>()
const labels: Record<Status, string> = {
  up: '在线',
  down: '离线',
  warning: '警告',
  unknown: '未知',
}
const types = { up: 'success', down: 'error', warning: 'warning', unknown: 'default' } as const
const current = computed(() => props.status ?? 'unknown')
</script>

<template>
  <NTag :type="types[current]" size="small" round :bordered="false">
    <span class="status-dot" :class="current" />{{ labels[current] }}
  </NTag>
</template>

<style scoped>
.status-dot {
  display: inline-block;
  width: 0.42rem;
  height: 0.42rem;
  margin-right: 0.4rem;
  border-radius: 50%;
  background: #64748b;
}

.status-dot.up { background: #34d399; box-shadow: 0 0 0.5rem #34d399; }
.status-dot.down { background: #fb7185; }
.status-dot.warning { background: #fbbf24; }
</style>

