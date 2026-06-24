import type { DashboardGroup } from '../types'
import { api } from './client'

export interface DashboardPayload {
  groups: DashboardGroup[]
}

export const dashboardApi = {
  get: () => api<DashboardPayload>('/api/dashboard'),
}
