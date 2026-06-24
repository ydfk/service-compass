import { defineStore } from 'pinia'
import { shallowRef } from 'vue'
import { dashboardApi } from '../api/dashboard'
import type { DashboardGroup } from '../types'

export const useDashboardStore = defineStore('dashboard', () => {
  const groups = shallowRef<DashboardGroup[]>([])
  const loading = shallowRef(false)

  async function load() {
    loading.value = true
    try {
      groups.value = (await dashboardApi.get()).groups
    } finally {
      loading.value = false
    }
  }

  return { groups, loading, load }
})
