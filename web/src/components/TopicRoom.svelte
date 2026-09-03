<script lang="ts">
  import type { Topic, TopicWorkspace } from '../lib/types';
  import Council from './Council.svelte';
  import IssueBoard from './IssueBoard.svelte';

  export let topic: Topic;
  export let workspace: TopicWorkspace;
</script>

<div class="topic-room page-enter">
  <main class="room-main">
    <header class="room-header">
      <div class="room-breadcrumb"><span>TOPIC</span><i>/</i><strong>{topic.key}</strong></div>
      <div class="room-title-row">
        <div><h1>{topic.title}</h1><p>{topic.description || 'A room for focused questions and durable decisions.'}</p></div>
        <button class="button button--primary" type="button" disabled><span>＋</span> New issue</button>
      </div>
      <nav class="room-tabs" aria-label="Topic views">
        <button class="active" type="button">Board <span>{workspace.issues.length}</span></button>
        <button type="button" disabled>Issues</button>
        <button type="button" disabled>Documents</button>
        <button type="button" disabled>Activity</button>
      </nav>
    </header>

    <div class="board-toolbar">
      <div class="view-switch"><button class="active" type="button">▦ Board</button><button type="button" disabled>☷ List</button></div>
      <div class="toolbar-actions"><button type="button" disabled>⌕ Filter</button><button type="button" disabled>⇅ Display</button></div>
    </div>

    <IssueBoard issues={workspace.issues} runs={workspace.runs} />
  </main>
  <Council
    members={workspace.members}
    teams={workspace.teams}
    aiMembers={workspace.aiMembers}
    models={workspace.models}
    runs={workspace.runs}
  />
</div>
