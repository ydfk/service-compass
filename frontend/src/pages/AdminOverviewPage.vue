<script setup lang="ts">
import { ArrowUpRight, BrandDocker, FileText, Stack2 } from '@vicons/tabler'
import { NCard, NIcon } from 'naive-ui'
import { RouterLink } from 'vue-router'

const shortcuts = [
  { title: '服务', copy: '统一配置入口、Docker 与监控', path: '/admin/services', icon: Stack2 },
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
</script>

<template>
  <header class="page-header"><p>CONTROL DECK</p><h1>管理概览</h1><span>配置你的服务航线与状态检查。</span></header>
  <div class="shortcut-grid">
    <RouterLink v-for="item in shortcuts" :key="item.path" :to="item.path">
      <NCard hoverable><NIcon class="shortcut-icon" :component="item.icon" /><h2>{{ item.title }}</h2><p>{{ item.copy }}</p><NIcon class="arrow" :component="ArrowUpRight" /></NCard>
    </RouterLink>
  </div>
</template>

<style scoped>
.page-header p { margin: 0; color: #5da9ff; font-family: "IBM Plex Mono", monospace; font-size: 0.68rem; letter-spacing: 0.2em; }
.page-header h1 { margin: 0.4rem 0; font-size: 2.4rem; }
.page-header span, .shortcut-grid p { color: #7b8ba1; }
.shortcut-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(14rem, 1fr)); gap: 1rem; margin-top: 2.5rem; }
.shortcut-grid a { color: inherit; text-decoration: none; }
.shortcut-grid h2 { margin: 1rem 0 0.3rem; }
.shortcut-grid p { margin: 0; }
.shortcut-icon { color: #5da9ff; font-size: 2rem; }
.arrow { float: right; color: #52627a; }
</style>
