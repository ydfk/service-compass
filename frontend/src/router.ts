import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from './stores/auth'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', component: () => import('./pages/DashboardPage.vue') },
    { path: '/login', component: () => import('./pages/LoginPage.vue') },
    {
      path: '/admin',
      component: () => import('./layouts/AdminLayout.vue'),
      meta: { requiresAuth: true },
      children: [
        { path: '', component: () => import('./pages/AdminOverviewPage.vue') },
        { path: 'services', component: () => import('./pages/ServicesPage.vue') },
        { path: 'discovery', redirect: '/admin/docker' },
        { path: 'docker', component: () => import('./pages/DockerDiscoveryPage.vue') },
        { path: 'monitors', component: () => import('./pages/MonitorsPage.vue') },
        { path: 'notify', component: () => import('./pages/NotificationsPage.vue') },
        { path: 'settings', component: () => import('./pages/SettingsPage.vue') },
        { path: 'settings/backup', component: () => import('./pages/BackupPage.vue') },
        { path: 'settings/docker', redirect: '/admin/docker' },
        { path: 'settings/logs', component: () => import('./pages/SystemLogsPage.vue') },
      ],
    },
  ],
})

router.beforeEach(async (to) => {
  if (!to.meta.requiresAuth) return true
  const auth = useAuthStore()
  return (await auth.verify()) ? true : { path: '/login', query: { redirect: to.fullPath } }
})

export default router
