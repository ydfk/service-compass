import { defineStore } from 'pinia'
import { shallowRef } from 'vue'
import { dashboardApi } from '../api/dashboard'
import type { DashboardGroup, DashboardSpace } from '../types'

export const useDashboardStore = defineStore('dashboard', () => {
  const spaces = shallowRef<DashboardSpace[]>([])
  const groups = shallowRef<DashboardGroup[]>([])
  const loading = shallowRef(false)
  const refreshIntervalSec = shallowRef(30)
  let refreshTimer: number | null = null

  async function load(options: { silent?: boolean } = {}) {
    if (!options.silent) loading.value = true
    try {
      const payload = await dashboardApi.get()
      spaces.value = payload.spaces
      groups.value = payload.groups
      const nextInterval = Math.max(5, Math.min(3600, payload.refresh_interval_sec || 30))
      const intervalChanged = refreshIntervalSec.value !== nextInterval
      refreshIntervalSec.value = nextInterval
      if (refreshTimer != null && intervalChanged) resetTimer()
    } finally {
      if (!options.silent) loading.value = false
    }
  }

  function startAutoRefresh() {
    stopAutoRefresh()
    resetTimer()
  }

  function resetTimer() {
    if (refreshTimer != null) window.clearInterval(refreshTimer)
    refreshTimer = window.setInterval(
      () => void load({ silent: true }),
      refreshIntervalSec.value * 1000,
    )
  }

  function stopAutoRefresh() {
    if (refreshTimer != null) window.clearInterval(refreshTimer)
    refreshTimer = null
  }

  return { spaces, groups, loading, refreshIntervalSec, load, startAutoRefresh, stopAutoRefresh }
})
