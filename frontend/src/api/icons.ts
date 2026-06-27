import { api } from './client'

export interface FaviconRequest {
  url: string
  auth_type?: 'none' | 'basic'
  auth_username?: string | null
  auth_password?: string | null
}

export const iconsApi = {
  suggest: (name: string) =>
    api<{ reference: string; urls: string[] }>(
      `/api/icons/suggest?name=${encodeURIComponent(name)}`,
    ),
  test: (reference: string, signal?: AbortSignal) =>
    api<{ ok: boolean; url: string }>(
      `/api/icons/test?reference=${encodeURIComponent(reference)}`,
      { signal },
    ),
  favicon: (body: FaviconRequest) =>
    api<{ urls: string[] }>('/api/icons/favicon', {
      method: 'POST',
      body: JSON.stringify(body),
    }),
  upload: (file: File) => {
    const body = new FormData()
    body.append('file', file)
    return api<{ url: string }>('/api/icons/upload', { method: 'POST', body })
  },
}
