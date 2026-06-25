<script setup lang="ts">
import {
  Bell,
  BrandDocker,
  Dashboard,
  FileText,
  HeartRateMonitor,
  Logout,
  Settings,
  Stack2,
} from '@vicons/tabler'
import { useMediaQuery } from '@vueuse/core'
import {
  NButton,
  NIcon,
  NLayout,
  NLayoutContent,
  NLayoutSider,
  NMenu,
  type MenuOption,
} from 'naive-ui'
import { h } from 'vue'
import { RouterLink, RouterView, useRoute, useRouter } from 'vue-router'
import ThemeToggle from '../components/ThemeToggle.vue'
import { useAuthStore } from '../stores/auth'

const route = useRoute()
const router = useRouter()
const auth = useAuthStore()
const mobile = useMediaQuery('(max-width: 760px)')

const menus: MenuOption[] = [
  item('概览', '/admin', Dashboard),
  item('服务', '/admin/services', Stack2),
  item('监控', '/admin/monitors', HeartRateMonitor),
  item('通知', '/admin/notify', Bell),
  {
    label: '设置',
    key: '/admin/settings-root',
    icon: () => h(NIcon, null, { default: () => h(Settings) }),
    children: [
      item('常规与账号', '/admin/settings', Settings),
      item('Docker', '/admin/settings/docker', BrandDocker),
      item('系统日志', '/admin/settings/logs', FileText),
    ],
  },
]

function item(label: string, path: string, icon: typeof Dashboard): MenuOption {
  return {
    label: () => h(RouterLink, { to: path }, { default: () => label }),
    key: path,
    icon: () => h(NIcon, null, { default: () => h(icon) }),
  }
}

async function logout() {
  await auth.logout()
  await router.replace('/login')
}
</script>

<template>
  <NLayout has-sider class="admin-shell">
    <NLayoutSider bordered :width="240" :collapsed-width="68" :collapsed="mobile" collapse-mode="width" :show-trigger="mobile ? false : 'bar'" class="sider">
      <RouterLink class="admin-brand" to="/"><img src="../assets/logo.svg" alt="" /><span>ServiceCompass<small>CONTROL DECK</small></span></RouterLink>
      <NMenu :value="route.path" :options="menus" />
      <div class="sider-footer">
        <ThemeToggle />
        <NButton quaternary @click="logout"><template #icon><NIcon :component="Logout" /></template>退出登录</NButton>
      </div>
    </NLayoutSider>
    <NLayoutContent class="admin-content"><RouterView /></NLayoutContent>
  </NLayout>
</template>

<style scoped>
.admin-shell { min-height: 100vh; }
.sider { position: fixed; z-index: 5; height: 100vh; background: var(--sc-bg-soft); }
.admin-brand { display: flex; height: 5.5rem; align-items: center; gap: 0.7rem; padding: 0 1.2rem; overflow: hidden; color: var(--sc-text); text-decoration: none; white-space: nowrap; }
.admin-brand img { width: 2.4rem; flex: 0 0 auto; }
.admin-brand span { display: grid; font-family: "IBM Plex Mono", monospace; font-size: 0.82rem; }
.admin-brand small { color: var(--sc-subtle); font-size: 0.56rem; letter-spacing: 0.13em; }
.sider-footer { position: absolute; bottom: 1.2rem; left: 1rem; display: grid; gap: 0.45rem; }
.admin-content { margin-left: 240px; min-height: 100vh; padding: 2.5rem; background: var(--sc-bg); }
@media (max-width: 760px) { .admin-content { margin-left: 68px; padding: 1.2rem; } }
</style>
