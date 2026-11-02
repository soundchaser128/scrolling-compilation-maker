import type { ClipCandidate, RenderRequest, RenderStatus } from './types';

async function req<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(path, {
    headers: { 'content-type': 'application/json' },
    ...init,
  });
  if (!res.ok) {
    const body = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(body.error ?? res.statusText);
  }
  return res.json() as Promise<T>;
}

export const api = {
  tags: (q: string) => req<string[]>(`/api/tags?query=${encodeURIComponent(q)}`),
  people: (q: string) => req<string[]>(`/api/people?query=${encodeURIComponent(q)}`),
  pool: (body: unknown) =>
    req<ClipCandidate[]>('/api/pool', { method: 'POST', body: JSON.stringify(body) }),
  render: (body: RenderRequest) =>
    req<{ id: string }>('/api/render', { method: 'POST', body: JSON.stringify(body) }),
  status: (id: string) => req<RenderStatus>(`/api/render/${id}`),
  outputUrl: (id: string) => `/api/output/${id}`,
  thumbUrl: (url: string, t = 1) => `/api/thumb?url=${encodeURIComponent(url)}&t=${t}`,
};
