import type {
  AiMember,
  AdminWorkspace,
  Comment,
  CommentKind,
  Creation,
  DiscoveredModel,
  Envelope,
  Issue,
  IssueType,
  Model,
  Principal,
  Provider,
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

async function response<T>(
  path: string,
  token: string,
  method = 'GET',
  body?: unknown
): Promise<T> {
  const response = await fetch(path, {
    method,
    headers: {
      Authorization: `Bearer ${token}`,
      ...(body === undefined ? {} : { 'Content-Type': 'application/json' })
    },
    body: body === undefined ? undefined : JSON.stringify(body)
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
  return await response.json() as T;
}

async function request<T>(path: string, token: string, method = 'GET', body?: unknown): Promise<T> {
  return (await response<Envelope<T>>(path, token, method, body)).data;
}

export function currentPrincipal(token: string): Promise<Principal> {
  return request('/api/v1/me', token);
}

export function listTopics(token: string): Promise<Topic[]> {
  return request('/api/v1/topics', token);
}

export function createTopic(
  token: string,
  input: { key: string; title: string; description: string }
): Promise<Topic> {
  return request('/api/v1/topics', token, 'POST', input);
}

export function listIssueTypes(token: string): Promise<IssueType[]> {
  return request('/api/v1/issue-types', token);
}

export function createIssue(
  token: string,
  topicId: string,
  input: { issue_type: string; title: string; body: string; parent_issue_id: string | null }
): Promise<Creation<Issue>> {
  return response(`/api/v1/topics/${topicId}/issues`, token, 'POST', input);
}

export function listComments(token: string, issueId: string): Promise<Comment[]> {
  return request(`/api/v1/issues/${issueId}/comments`, token);
}

export function createComment(
  token: string,
  issueId: string,
  input: { kind: CommentKind; body: string; reply_to_comment_id: string | null }
): Promise<Creation<Comment>> {
  return response(`/api/v1/issues/${issueId}/comments`, token, 'POST', input);
}

export async function loadAdminWorkspace(token: string): Promise<AdminWorkspace> {
  const [providers, models, aiMembers] = await Promise.all([
    request<Provider[]>('/api/v1/providers', token),
    request<Model[]>('/api/v1/models', token),
    request<AiMember[]>('/api/v1/ai-members', token)
  ]);
  return { providers, models, aiMembers };
}

export function createProvider(
  token: string,
  input: {
    name: string;
    adapter: 'openai_compatible';
    base_url: string;
    credential_ref?: string;
    api_key?: string;
  }
): Promise<Provider> {
  return request('/api/v1/providers', token, 'POST', input);
}

export function discoverProviderModels(token: string, providerId: string): Promise<DiscoveredModel[]> {
  return request(`/api/v1/providers/${providerId}/models`, token);
}

export function createModel(
  token: string,
  input: {
    provider_id: string;
    model_name: string;
    display_name: string;
    capabilities?: Record<string, unknown>;
    limits?: Record<string, unknown>;
    defaults?: Record<string, unknown>;
  }
): Promise<Model> {
  return request('/api/v1/models', token, 'POST', input);
}

export function createAiMember(
  token: string,
  input: { handle: string; display_name: string; identity_prompt: string; provider_id: string; model_name: string }
): Promise<AiMember> {
  return request('/api/v1/ai-members', token, 'POST', input);
}

export function addTopicMember(
  token: string,
  topicId: string,
  principalId: string,
  role: 'read' | 'contribute' | 'write' = 'contribute'
): Promise<TopicMember> {
  return request(`/api/v1/topics/${topicId}/members/${principalId}`, token, 'PUT', { role });
}

export function createTeam(
  token: string,
  topicId: string,
  input: { handle: string; display_name: string }
): Promise<Team> {
  return request(`/api/v1/topics/${topicId}/teams`, token, 'POST', input);
}

export function addTeamMember(token: string, teamId: string, principalId: string): Promise<Team> {
  return request(`/api/v1/teams/${teamId}/members/${principalId}`, token, 'PUT');
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
