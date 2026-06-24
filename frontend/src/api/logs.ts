import { api } from './client'

export interface LogEntry {
  line: string
}

export const logsApi = {
  list: (limit = 300) => api<{ logs: LogEntry[] }>(`/api/logs?limit=${limit}`),
}
