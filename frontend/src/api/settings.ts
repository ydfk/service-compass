import { api } from './client'

export interface SettingsPayload {
  retention_days: number
  log_retention_days: number
  cert_expiry_warning_days: number
  notification_cooldown_sec: number
}

export const settingsApi = {
  get: () => api<SettingsPayload>('/api/settings'),
  update: (input: SettingsPayload) =>
    api<SettingsPayload>('/api/settings', {
      method: 'PUT',
      body: JSON.stringify(input),
    }),
}
