<script lang="ts">
  import { addTeamMember, addTopicMember, createTeam } from '../lib/api';
  import type { Creation, Issue, IssueType, Topic, TopicWorkspace } from '../lib/types';
  import Council from './Council.svelte';
  import IssueBoard from './IssueBoard.svelte';
  import IssueComposer from './IssueComposer.svelte';
  import IssueDetail from './IssueDetail.svelte';

  export let token: string;
  export let topic: Topic;
  export let workspace: TopicWorkspace;
  export let issueTypes: IssueType[];
  export let onRefresh: () => Promise<void>;

  let composing = false;
  let selectedIssueId: string | null = null;
  $: selectedIssue = workspace.issues.find((issue) => issue.id === selectedIssueId) ?? null;

  async function created(creation: Creation<Issue>) {
    composing = false;
    selectedIssueId = creation.data.id;
    await onRefresh();
  }

  async function addMember(memberId: string) {
    await addTopicMember(token, topic.id, memberId);
    await onRefresh();
  }

  async function addTeam(input: { handle: string; displayName: string; memberId: string }) {
    await addTopicMember(token, topic.id, input.memberId);
    const team = await createTeam(token, topic.id, { handle: input.handle, display_name: input.displayName });
    await addTeamMember(token, team.id, input.memberId);
    await onRefresh();
  }
</script>

<div class="topic-room page-enter">
  <main class="room-main">
    <header class="room-header">
      <div class="room-breadcrumb"><span>TOPIC</span><i>/</i><strong>{topic.key}</strong></div>
      <div class="room-title-row">
        <div><h1>{topic.title}</h1><p>{topic.description || 'A room for focused questions and durable decisions.'}</p></div>
        <button class="button button--primary" type="button" on:click={() => composing = true}><span>＋</span> New issue</button>
      </div>
      <nav class="room-tabs" aria-label="Topic views">
        <button class="active" type="button">Board <span>{workspace.issues.length}</span></button>
        <button type="button" disabled>Issues</button>
        <button type="button" disabled>Documents</button>
        <button type="button" disabled>Activity</button>
      </nav>
    </header>

    {#if selectedIssue}
      <IssueDetail {token} issue={selectedIssue} members={workspace.members} aiMembers={workspace.aiMembers} teams={workspace.teams} runs={workspace.runs} onBack={() => selectedIssueId = null} {onRefresh} />
    {:else}
      <div class="board-toolbar">
        <div class="view-switch"><button class="active" type="button">▦ Board</button><button type="button" disabled>☷ List</button></div>
        <div class="toolbar-actions"><button type="button" disabled>⌕ Filter</button><button type="button" disabled>⇅ Display</button></div>
      </div>
      <IssueBoard issues={workspace.issues} runs={workspace.runs} onSelect={(issue) => selectedIssueId = issue.id} />
    {/if}
  </main>
  <Council
    members={workspace.members}
    teams={workspace.teams}
    aiMembers={workspace.aiMembers}
    models={workspace.models}
    runs={workspace.runs}
    onAddMember={addMember}
    onCreateTeam={addTeam}
  />
</div>

{#if composing}
  <IssueComposer {token} topicId={topic.id} {issueTypes} members={workspace.members} teams={workspace.teams} onClose={() => composing = false} onCreated={created} />
{/if}
