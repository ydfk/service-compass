import { api } from './client'

export const iconsApi = {
  suggest: (name: string) =>
    api<{ reference: string; urls: string[] }>(
      `/api/icons/suggest?name=${encodeURIComponent(name)}`,
    ),
  test: (reference: string) =>
    api<{ ok: boolean; url: string }>(`/api/icons/test?reference=${encodeURIComponent(reference)}`),
  favicon: (url: string) =>
    api<{ url: string }>(`/api/icons/favicon?url=${encodeURIComponent(url)}`),
  upload: (file: File) => {
    const body = new FormData()
    body.append('file', file)
    return api<{ url: string }>('/api/icons/upload', { method: 'POST', body })
  },
}
