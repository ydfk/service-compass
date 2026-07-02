import type { Service, ServiceInput } from '../types'
import { api } from './client'

export const servicesApi = {
  list: () => api<Service[]>('/api/services'),
  create: (input: ServiceInput) => api<Service>('/api/services', request('POST', input)),
  update: (id: string, input: ServiceInput) =>
    api<Service>(`/api/services/${id}`, request('PUT', input)),
  remove: (id: string) => api<{ ok: boolean }>(`/api/services/${id}`, { method: 'DELETE' }),
  reorder: (items: Array<{ id: string; sort_order: number }>) =>
    api<{ ok: boolean }>('/api/services/reorder', {
      method: 'POST',
      body: JSON.stringify(items),
    }),
  openUrl: (id: string, mode: 'local' | 'public') =>
    api<{ url?: string | null }>(`/api/services/${id}/test-open?mode=${mode}`, { method: 'POST' }),
}

function request(method: string, input: ServiceInput): RequestInit {
  return { method, body: JSON.stringify(input) }
}
