import { api } from './client'

export interface VersionInfo {
  current_version: string
  latest_version?: string | null
  update_available: boolean
  release_url?: string | null
}

export const versionApi = {
  get: () => api<VersionInfo>('/api/version'),
}
