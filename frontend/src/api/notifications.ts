import type { NotificationChannel, NotificationRule, NotificationRuleInput } from '../types'
import { api } from './client'

export interface NotificationChannelInput {
  name: string
  channel_type: NotificationChannel['channel_type']
  enabled: boolean
  config?: Record<string, unknown>
}

export const notificationsApi = {
  channels: () => api<NotificationChannel[]>('/api/notifications/channels'),
  createChannel: (input: NotificationChannelInput) =>
    api<NotificationChannel>('/api/notifications/channels', request('POST', input)),
  updateChannel: (id: string, input: NotificationChannelInput) =>
    api<NotificationChannel>(`/api/notifications/channels/${id}`, request('PUT', input)),
  removeChannel: (id: string) =>
    api<{ ok: boolean }>(`/api/notifications/channels/${id}`, { method: 'DELETE' }),
  testChannel: (id: string) =>
    api<{ status_code: number; response_summary: string }>(
      `/api/notifications/channels/${id}/test`,
      { method: 'POST' },
    ),
  rules: () => api<NotificationRule[]>('/api/notifications/rules'),
  createRule: (input: NotificationRuleInput) =>
    api<NotificationRule>('/api/notifications/rules', request('POST', input)),
  updateRule: (id: string, input: NotificationRuleInput) =>
    api<NotificationRule>(`/api/notifications/rules/${id}`, request('PUT', input)),
  removeRule: (id: string) =>
    api<{ ok: boolean }>(`/api/notifications/rules/${id}`, { method: 'DELETE' }),
}

function request(method: string, input: unknown): RequestInit {
  return { method, body: JSON.stringify(input) }
}
