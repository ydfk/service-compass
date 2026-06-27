import type { Monitor, MonitorCheck, MonitorInput, MonitorNotificationInput } from '../types'
import { api } from './client'

export interface CheckResult {
  status: string
  latency_ms?: number | null
  status_code?: number | null
  error_message?: string | null
  extra_json?: string | null
}

export const monitorsApi = {
  list: () => api<Monitor[]>('/api/monitors'),
  create: (input: MonitorInput) => api<Monitor>('/api/monitors', request('POST', input)),
  update: (id: string, input: MonitorInput) =>
    api<Monitor>(`/api/monitors/${id}`, request('PUT', input)),
  updateNotification: (id: string, input: MonitorNotificationInput) =>
    api<Monitor>(`/api/monitors/${id}/notification`, {
      method: 'PATCH',
      body: JSON.stringify(input),
    }),
  remove: (id: string) => api<{ ok: boolean }>(`/api/monitors/${id}`, { method: 'DELETE' }),
  test: (id: string) => api<CheckResult>(`/api/monitors/${id}/test`, { method: 'POST' }),
  checks: (id: string) => api<MonitorCheck[]>(`/api/monitors/${id}/checks?limit=500`),
}

function request(method: string, input: MonitorInput): RequestInit {
  return { method, body: JSON.stringify(input) }
}
