import type { DashboardGroup, DashboardSpace } from '../types'
import { api } from './client'

export interface DashboardPayload {
  spaces: DashboardSpace[]
  groups: DashboardGroup[]
  refresh_interval_sec: number
}

export interface DashboardSummary {
  total: number
  up: number
  down: number
  warning: number
  unknown: number
  monitors: number
  checks_24h: number
  avg_latency_ms?: number | null
}

export const dashboardApi = {
  get: () => api<DashboardPayload>('/api/dashboard'),
  summary: () => api<DashboardSummary>('/api/dashboard/summary'),
}
