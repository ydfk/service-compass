<script setup lang="ts">
import { ArrowUpRight, BrandDocker, FileText, HeartRateMonitor, Stack2 } from '@vicons/tabler'
import { NCard, NIcon, NSpin } from 'naive-ui'
import { computed, onMounted, ref } from 'vue'
import { RouterLink } from 'vue-router'
import { dashboardApi, type DashboardSummary } from '../api/dashboard'

const summary = ref<DashboardSummary | null>(null)
const loading = ref(false)
const availability = computed(() => {
  if (!summary.value?.total) return '—'
  return `${((summary.value.up / summary.value.total) * 100).toFixed(1)}%`
})

const shortcuts = [
  { title: '服务', copy: '统一配置入口、Docker 与监控', path: '/admin/services', icon: Stack2 },
  {
    title: '监控',
    copy: '按分组和服务查看检查日志',
    path: '/admin/monitors',
    icon: HeartRateMonitor,
  },
  {
    title: 'Docker 发现',
    copy: '扫描候选服务，确认后添加',
    path: '/admin/settings/docker',
    icon: BrandDocker,
  },
  {
    title: '系统日志',
    copy: '按时间倒序查看运行记录',
    path: '/admin/settings/logs',
    icon: FileText,
  },
]

onMounted(async () => {
  loading.value = true
  try {
    summary.value = await dashboardApi.summary()
  } finally {
    loading.value = false
  }
})
</script>

<template>
  <header class="page-header"><p>CONTROL DECK</p><h1>管理概览</h1><span>优先关注服务可用性、告警和检查吞吐。</span></header>
  <NSpin :show="loading">
    <div class="stat-grid">
      <NCard title="服务可用率"><strong>{{ availability }}</strong><span>{{ summary?.up ?? 0 }} / {{ summary?.total ?? 0 }} 在线</span></NCard>
      <NCard title="异常服务"><strong class="danger">{{ summary?.down ?? 0 }}</strong><span>down 状态需要优先处理</span></NCard>
      <NCard title="告警服务"><strong class="warning">{{ summary?.warning ?? 0 }}</strong><span>包含证书临期与 Docker starting</span></NCard>
      <NCard title="监控器"><strong>{{ summary?.monitors ?? 0 }}</strong><span>24h 检查 {{ summary?.checks_24h ?? 0 }} 次</span></NCard>
      <NCard title="平均延迟"><strong>{{ summary?.avg_latency_ms == null ? '—' : `${Math.round(summary.avg_latency_ms)} ms` }}</strong><span>来自最近一次监控状态</span></NCard>
      <NCard title="未知状态"><strong>{{ summary?.unknown ?? 0 }}</strong><span>等待首次检查或未配置监控</span></NCard>
    </div>
  </NSpin>
  <div class="shortcut-grid">
    <RouterLink v-for="item in shortcuts" :key="item.path" :to="item.path">
      <NCard hoverable><NIcon class="shortcut-icon" :component="item.icon" /><h2>{{ item.title }}</h2><p>{{ item.copy }}</p><NIcon class="arrow" :component="ArrowUpRight" /></NCard>
    </RouterLink>
  </div>
</template>

<style scoped>
.page-header p { margin: 0; color: #5da9ff; font-family: "IBM Plex Mono", monospace; font-size: 0.68rem; letter-spacing: 0.2em; }
.page-header h1 { margin: 0.4rem 0; font-size: 2.4rem; }
.page-header span, .shortcut-grid p, .stat-grid span { color: var(--sc-muted); }
.stat-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr)); gap: 0.9rem; margin-top: 1.6rem; }
.stat-grid strong { display: block; margin-bottom: 0.25rem; font-family: "IBM Plex Mono", monospace; font-size: 2rem; line-height: 1; }
.stat-grid span { font-size: 0.78rem; }
.danger { color: #fb7185; }
.warning { color: #f59e0b; }
.shortcut-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(14rem, 1fr)); gap: 1rem; margin-top: 1.4rem; }
.shortcut-grid a { color: inherit; text-decoration: none; }
.shortcut-grid h2 { margin: 1rem 0 0.3rem; }
.shortcut-grid p { margin: 0; }
.shortcut-icon { color: #5da9ff; font-size: 2rem; }
.arrow { float: right; color: #52627a; }
</style>
