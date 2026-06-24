import type { Group } from '../types'
import { api } from './client'

export interface GroupInput {
  name: string
  description?: string | null
  icon?: string | null
  sort_order: number
}

export const groupsApi = {
  list: () => api<Group[]>('/api/groups'),
  create: (input: GroupInput) => api<Group>('/api/groups', request('POST', input)),
  update: (id: string, input: GroupInput) => api<Group>(`/api/groups/${id}`, request('PUT', input)),
  remove: (id: string) => api<{ ok: boolean }>(`/api/groups/${id}`, { method: 'DELETE' }),
}

function request(method: string, input: GroupInput): RequestInit {
  return { method, body: JSON.stringify(input) }
}
