interface ApiErrorBody {
  error?: { message?: string }
}

export class ApiError extends Error {
  constructor(
    message: string,
    readonly status: number,
  ) {
    super(message)
  }
}

export async function api<T>(path: string, init: RequestInit = {}): Promise<T> {
  const token = localStorage.getItem('service-compass-session')
  const headers = new Headers(init.headers)
  if (init.body && !(init.body instanceof FormData)) headers.set('Content-Type', 'application/json')
  if (token) headers.set('Authorization', `Bearer ${token}`)

  const response = await fetch(path, { ...init, headers })
  if (!response.ok) {
    const body = (await response.json().catch(() => ({}))) as ApiErrorBody
    if (response.status === 401) localStorage.removeItem('service-compass-session')
    throw new ApiError(body.error?.message ?? `请求失败 (${response.status})`, response.status)
  }
  return (await response.json()) as T
}
