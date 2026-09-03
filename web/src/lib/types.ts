export type PrincipalKind = 'human' | 'ai' | 'caller' | 'system';
export type IssueState = 'open' | 'closed';

export interface Principal {
  id: string;
  kind: PrincipalKind;
  handle: string;
  display_name: string;
}

export interface Topic {
  id: string;
  key: string;
  title: string;
  description: string;
  revision: number;
}

export interface Issue {
  id: string;
  topic_id: string;
  number: number;
  issue_type: string;
  state: IssueState;
  title: string;
  body: string;
  parent_issue_id: string | null;
  author_id: string;
  revision: number;
}

export interface TopicMember extends Principal {
  role: 'read' | 'contribute' | 'write';
}

export interface Team {
  id: string;
  topic_id: string;
  handle: string;
  display_name: string;
  members: string[];
}

export interface AiMember extends Principal {
  identity_prompt_version: number;
  default_model_id: string;
}

export interface Model {
  id: string;
  provider_id: string;
  model_name: string;
  display_name: string;
  enabled: boolean;
}

export interface Provider {
  id: string;
  name: string;
  adapter: 'openai_compatible';
  base_url: string;
  credential_configured: boolean;
  enabled: boolean;
}

export interface DiscoveredModel {
  id: string;
  owned_by: string | null;
}

export interface IssueType {
  key: string;
  display_name: string;
  description: string;
}

export type CommentKind = 'discussion' | 'direction' | 'evidence' | 'progress' | 'result';

export interface Comment {
  id: string;
  item_id: string;
  author_id: string;
  kind: CommentKind;
  body: string;
  revision: number;
  reply_to_comment_id: string | null;
}

export interface DispatchSummary {
  id: string;
  status: 'pending';
}

export interface Creation<T> {
  data: T;
  dispatch: DispatchSummary | null;
}

export interface Run {
  id: string;
  item_id: string;
  ai_member_id: string;
  model_id: string;
  status: 'queued' | 'in_progress' | 'completed';
  conclusion: 'success' | 'failure' | 'cancelled' | 'timed_out' | 'skipped' | 'neutral' | null;
}

export interface Envelope<T> {
  data: T;
}

export interface TopicWorkspace {
  issues: Issue[];
  members: TopicMember[];
  teams: Team[];
  aiMembers: AiMember[];
  models: Model[];
  runs: Run[];
}

export interface AdminWorkspace {
  providers: Provider[];
  models: Model[];
  aiMembers: AiMember[];
}
