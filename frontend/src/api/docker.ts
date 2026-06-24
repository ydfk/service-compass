import type { DockerCandidate, DockerEndpoint, DockerEndpointInput } from '../types'
import { api } from './client'

export interface AddCandidateInput {
  endpoint_id: string
  container_id: string
  group_id: string
  name?: string | null
  local_url?: string | null
  public_url?: string | null
  icon_value?: string | null
  create_monitor: boolean
}

export const dockerApi = {
  endpoints: () => api<DockerEndpoint[]>('/api/docker/endpoints'),
  createEndpoint: (input: DockerEndpointInput) =>
    api<DockerEndpoint>('/api/docker/endpoints', request('POST', input)),
  updateEndpoint: (id: string, input: DockerEndpointInput) =>
    api<DockerEndpoint>(`/api/docker/endpoints/${id}`, request('PUT', input)),
  removeEndpoint: (id: string) =>
    api<{ ok: boolean }>(`/api/docker/endpoints/${id}`, { method: 'DELETE' }),
  testEndpoint: (id: string) =>
    api<{ ok: boolean }>(`/api/docker/endpoints/${id}/test`, { method: 'POST' }),
  scan: (id: string) =>
    api<DockerCandidate[]>(`/api/docker/endpoints/${id}/scan`, { method: 'POST' }),
  candidates: (id: string) => api<DockerCandidate[]>(`/api/docker/endpoints/${id}/candidates`),
  addCandidate: (input: AddCandidateInput) =>
    api<{ service_id: string }>('/api/docker/candidates/add', request('POST', input)),
}

function request(method: string, input: unknown): RequestInit {
  return { method, body: JSON.stringify(input) }
}
