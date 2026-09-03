import type {
  AiMember,
  Envelope,
  Issue,
  Model,
  Principal,
  Run,
  Team,
  Topic,
  TopicMember,
  TopicWorkspace
} from './types';

export class ApiError extends Error {
  constructor(
    public readonly status: number,
    message: string
  ) {
    super(message);
  }
}

async function request<T>(path: string, token: string): Promise<T> {
  const response = await fetch(path, {
    headers: { Authorization: `Bearer ${token}` }
  });
  if (!response.ok) {
    let message = `Request failed with HTTP ${response.status}`;
    try {
      const payload = await response.json();
      message = payload.error?.message ?? message;
    } catch {
      // Preserve the status-based fallback for non-JSON failures.
    }
    throw new ApiError(response.status, message);
  }
  return (await response.json() as Envelope<T>).data;
}

export function currentPrincipal(token: string): Promise<Principal> {
  return request('/api/v1/me', token);
}

export function listTopics(token: string): Promise<Topic[]> {
  return request('/api/v1/topics', token);
}

export async function loadTopicWorkspace(
  token: string,
  topicId: string
): Promise<TopicWorkspace> {
  const [issues, members, teams, aiMembers, models, runs] = await Promise.all([
    request<Issue[]>(`/api/v1/topics/${topicId}/issues`, token),
    request<TopicMember[]>(`/api/v1/topics/${topicId}/members`, token),
    request<Team[]>(`/api/v1/topics/${topicId}/teams`, token),
    optionalAdminList<AiMember>('/api/v1/ai-members', token),
    optionalAdminList<Model>('/api/v1/models', token),
    request<Run[]>(`/api/v1/topics/${topicId}/runs`, token)
  ]);
  return { issues, members, teams, aiMembers, models, runs };
}

async function optionalAdminList<T>(path: string, token: string): Promise<T[]> {
  try {
    return await request<T[]>(path, token);
  } catch (error) {
    if (error instanceof ApiError && error.status === 403) return [];
    throw error;
  }
}
