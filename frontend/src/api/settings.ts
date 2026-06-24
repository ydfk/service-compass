import { api } from './client'

export const settingsApi = {
  get: () => api<{ retention_days: number }>('/api/settings'),
  update: (retention_days: number) =>
    api<{ retention_days: number }>('/api/settings', {
      method: 'PUT',
      body: JSON.stringify({ retention_days }),
    }),
}
