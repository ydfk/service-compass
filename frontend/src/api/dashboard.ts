import type { DashboardGroup } from '../types'
import { api } from './client'

export interface DashboardPayload {
  groups: DashboardGroup[]
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
