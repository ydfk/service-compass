<script setup lang="ts">
import { Refresh } from '@vicons/tabler'
import { NButton, NCard, NEmpty, NIcon, NInput, NSpace, NSpin } from 'naive-ui'
import { computed, onMounted, ref } from 'vue'
import { logsApi, type LogEntry } from '../api/logs'

const logs = ref<LogEntry[]>([])
const loading = ref(false)
const search = ref('')
const filteredLogs = computed(() => {
  const keyword = search.value.trim().toLowerCase()
  if (!keyword) return logs.value
  return logs.value.filter((item) => item.line.toLowerCase().includes(keyword))
})

async function load() {
  loading.value = true
  try {
    logs.value = (await logsApi.list()).logs
  } finally {
    loading.value = false
  }
}

onMounted(load)
</script>

<template>
  <header class="page-header"><div><p>SYSTEM LOGS</p><h1>系统日志</h1><span>最新记录优先，仅保留当前进程最近的 1000 条。</span></div><NButton @click="load"><template #icon><NIcon :component="Refresh" /></template>刷新</NButton></header>
  <NSpace class="filter-bar">
    <NInput v-model:value="search" clearable placeholder="搜索日志内容" class="log-search" />
    <NButton secondary @click="search = ''">清除筛选</NButton>
  </NSpace>
  <NSpin :show="loading"><NCard class="log-card" :bordered="false"><NEmpty v-if="!filteredLogs.length" description="暂无日志" /><ol v-else><li v-for="(item, index) in filteredLogs" :key="`${index}-${item.line}`">{{ item.line }}</li></ol></NCard></NSpin>
</template>

<style scoped>
.page-header { display: flex; align-items: end; justify-content: space-between; margin-bottom: 1.5rem; }
.page-header p { margin: 0; color: #5da9ff; font-family: "IBM Plex Mono", monospace; font-size: 0.68rem; letter-spacing: 0.2em; }
.page-header h1 { margin: 0.3rem 0; font-size: 2.1rem; }
.page-header span { color: #75859b; }
.filter-bar { width: 100%; margin-bottom: 0.8rem; }
.log-search { width: min(42rem, 100%); flex: 1 1 24rem; }
.log-card { background: #060a11; }
ol { max-height: calc(100vh - 13rem); margin: 0; padding: 0; overflow: auto; list-style: none; }
li { padding: 0.52rem 0.2rem; border-bottom: 1px solid rgb(148 163 184 / 9%); color: #a9b5c7; font-family: "IBM Plex Mono", monospace; font-size: 0.72rem; line-height: 1.5; overflow-wrap: anywhere; }
</style>
