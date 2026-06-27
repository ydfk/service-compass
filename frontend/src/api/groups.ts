import type { Group, Space } from '../types'
import { api } from './client'

export interface GroupInput {
  space_id?: string | null
  name: string
  description?: string | null
  icon?: string | null
  sort_order: number
}

export interface SpaceInput {
  name: string
  description?: string | null
  sort_order: number
}

export const groupsApi = {
  spaces: () => api<Space[]>('/api/spaces'),
  createSpace: (input: SpaceInput) => api<Space>('/api/spaces', request('POST', input)),
  updateSpace: (id: string, input: SpaceInput) =>
    api<Space>(`/api/spaces/${id}`, request('PUT', input)),
  removeSpace: (id: string) => api<{ ok: boolean }>(`/api/spaces/${id}`, { method: 'DELETE' }),
  reorderSpaces: (items: Array<{ id: string; sort_order: number }>) =>
    api<{ ok: boolean }>('/api/spaces/reorder', {
      method: 'POST',
      body: JSON.stringify(items),
    }),
  list: () => api<Group[]>('/api/groups'),
  create: (input: GroupInput) => api<Group>('/api/groups', request('POST', input)),
  update: (id: string, input: GroupInput) => api<Group>(`/api/groups/${id}`, request('PUT', input)),
  remove: (id: string) => api<{ ok: boolean }>(`/api/groups/${id}`, { method: 'DELETE' }),
  reorder: (items: Array<{ id: string; sort_order: number }>) =>
    api<{ ok: boolean }>('/api/groups/reorder', {
      method: 'POST',
      body: JSON.stringify(items),
    }),
}

function request(method: string, input: GroupInput | SpaceInput): RequestInit {
  return { method, body: JSON.stringify(input) }
}
