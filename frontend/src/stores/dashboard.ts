import { defineStore } from 'pinia'
import { shallowRef } from 'vue'
import { dashboardApi } from '../api/dashboard'
import type { DashboardGroup, DashboardSpace } from '../types'

export const useDashboardStore = defineStore('dashboard', () => {
  const spaces = shallowRef<DashboardSpace[]>([])
  const groups = shallowRef<DashboardGroup[]>([])
  const loading = shallowRef(false)

  async function load() {
    loading.value = true
    try {
      const payload = await dashboardApi.get()
      spaces.value = payload.spaces
      groups.value = payload.groups
    } finally {
      loading.value = false
    }
  }

  return { spaces, groups, loading, load }
})
