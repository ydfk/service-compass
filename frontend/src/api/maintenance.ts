import type { BackupRun } from '../types'
import { api } from './client'

export interface BackupConfig {
  enabled: boolean
  schedule_time: string
  target_type: 'local' | 'webdav' | 'aliyun_oss'
  local_dir?: string | null
  webdav_url?: string | null
  webdav_username?: string | null
  webdav_password?: string | null
  has_webdav_password?: boolean
  aliyun_oss_endpoint?: string | null
  aliyun_oss_region?: string | null
  aliyun_oss_bucket?: string | null
  aliyun_oss_prefix?: string | null
  aliyun_oss_access_key_id?: string | null
  aliyun_oss_access_key_secret?: string | null
  has_aliyun_oss_access_key_secret?: boolean
  retention_count: number
  last_run_at?: string | null
}

export interface IconLocalizeResult {
  success: number
  failed: Array<{ service_id: string; reason: string }>
}

export interface BackupRunsPage {
  items: BackupRun[]
  total: number
}

export const maintenanceApi = {
  backupConfig: () => api<BackupConfig>('/api/maintenance/backup-config'),
  updateBackupConfig: (input: BackupConfig) =>
    api<BackupConfig>('/api/maintenance/backup-config', {
      method: 'PUT',
      body: JSON.stringify(input),
    }),
  runBackup: () =>
    api<{ ok: boolean; path: string }>('/api/maintenance/backup/run', { method: 'POST' }),
  testBackupTarget: (input: BackupConfig) =>
    api<{ ok: boolean; path: string }>('/api/maintenance/backup/test-target', {
      method: 'POST',
      body: JSON.stringify(input),
    }),
  backupRuns: (page = 1, pageSize = 20) =>
    api<BackupRunsPage>(`/api/maintenance/backup-runs?page=${page}&page_size=${pageSize}`),
  localizeIcons: () =>
    api<IconLocalizeResult>('/api/maintenance/icons/localize', { method: 'POST' }),
  importConfig: (file: File) => {
    const form = new FormData()
    form.append('file', file)
    return api<{ ok: boolean; restart_required: boolean; message: string }>(
      '/api/maintenance/import',
      { method: 'POST', body: form },
    )
  },
  exportConfig,
}

async function exportConfig() {
  const token = localStorage.getItem('service-compass-session')
  const headers = new Headers()
  if (token) headers.set('Authorization', `Bearer ${token}`)
  const response = await fetch('/api/maintenance/export', { headers })
  if (!response.ok) throw new Error(`导出失败 (${response.status})`)
  const blob = await response.blob()
  const filename = filenameFromDisposition(response.headers.get('content-disposition'))
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = filename
  link.click()
  URL.revokeObjectURL(url)
}

function filenameFromDisposition(value: string | null) {
  const matched = value?.match(/filename="([^"]+)"/)
  return matched?.[1] || 'service-compass-export.zip'
}
