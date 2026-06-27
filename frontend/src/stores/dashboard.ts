import { defineStore } from 'pinia'
import { shallowRef } from 'vue'
import { dashboardApi } from '../api/dashboard'
import type { DashboardGroup, DashboardSpace } from '../types'

export const useDashboardStore = defineStore('dashboard', () => {
  const spaces = shallowRef<DashboardSpace[]>([])
  const groups = shallowRef<DashboardGroup[]>([])
  const loading = shallowRef(false)
  let eventSource: EventSource | null = null
  let fallbackTimer: number | null = null

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

  function startAutoRefresh() {
    stopAutoRefresh()
    if (!('EventSource' in window)) {
      startFallbackPolling()
      return
    }
    eventSource = new EventSource('/api/dashboard/events')
    eventSource.onmessage = () => void load()
    eventSource.addEventListener('dashboard', () => void load())
    eventSource.onerror = () => {
      eventSource?.close()
      eventSource = null
      startFallbackPolling()
    }
  }

  function startFallbackPolling() {
    if (fallbackTimer != null) return
    fallbackTimer = window.setInterval(() => void load(), 30_000)
  }

  function stopAutoRefresh() {
    eventSource?.close()
    eventSource = null
    if (fallbackTimer != null) window.clearInterval(fallbackTimer)
    fallbackTimer = null
  }

  return { spaces, groups, loading, load, startAutoRefresh, stopAutoRefresh }
})
