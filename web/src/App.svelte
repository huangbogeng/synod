<script lang="ts">
  import { onMount } from 'svelte';
  import { ApiError, currentPrincipal, listIssueTypes, listTopics, loadTopicWorkspace } from './lib/api';
  import type { IssueType, Principal, Topic, TopicWorkspace } from './lib/types';
  import CreateTopic from './components/CreateTopic.svelte';
  import Login from './components/Login.svelte';
  import Overview from './components/Overview.svelte';
  import Sidebar from './components/Sidebar.svelte';
  import Settings from './components/Settings.svelte';
  import TopicRoom from './components/TopicRoom.svelte';

  const SESSION_KEY = 'synod.session.token';
  let token = '';
  let principal: Principal | null = null;
  let topics: Topic[] = [];
  let selectedId: string | null = null;
  let workspace: TopicWorkspace | null = null;
  let issueTypes: IssueType[] = [];
  let view: 'overview' | 'topic' | 'settings' = 'overview';
  let creatingTopic = false;
  let loading = false;
  let error = '';

  $: selectedTopic = topics.find((topic) => topic.id === selectedId) ?? null;

  onMount(() => {
    const saved = sessionStorage.getItem(SESSION_KEY);
    if (saved) void unlock(saved);
    const timer = window.setInterval(() => {
      if (view === 'topic' && selectedId && !loading) void refreshWorkspace();
    }, 2500);
    return () => window.clearInterval(timer);
  });

  async function unlock(candidate: string): Promise<boolean> {
    loading = true;
    error = '';
    try {
      const [nextPrincipal, nextTopics, nextIssueTypes] = await Promise.all([
        currentPrincipal(candidate),
        listTopics(candidate),
        listIssueTypes(candidate)
      ]);
      token = candidate;
      principal = nextPrincipal;
      topics = nextTopics;
      issueTypes = nextIssueTypes;
      sessionStorage.setItem(SESSION_KEY, candidate);
      return true;
    } catch (cause) {
      if (!(cause instanceof ApiError && cause.status === 401)) {
        error = cause instanceof Error ? cause.message : 'Synod could not be reached.';
      }
      return false;
    } finally {
      loading = false;
    }
  }

  async function selectTopic(id: string) {
    if (selectedId === id && workspace && view === 'topic') return;
    selectedId = id;
    view = 'topic';
    workspace = null;
    loading = true;
    error = '';
    try {
      workspace = await loadTopicWorkspace(token, id);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'The Topic could not be loaded.';
    } finally {
      loading = false;
    }
  }

  async function refreshWorkspace() {
    if (!selectedId) return;
    const refreshingId = selectedId;
    try {
      const next = await loadTopicWorkspace(token, refreshingId);
      if (selectedId === refreshingId) workspace = next;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'The Topic could not be refreshed.';
    }
  }

  function home() {
    selectedId = null;
    workspace = null;
    view = 'overview';
    error = '';
  }

  function logout() {
    sessionStorage.removeItem(SESSION_KEY);
    token = '';
    principal = null;
    topics = [];
    home();
  }


  function settings() {
    view = 'settings';
    error = '';
  }

  async function topicCreated(topic: Topic) {
    creatingTopic = false;
    topics = [topic, ...topics.filter((candidate) => candidate.id !== topic.id)];
    await selectTopic(topic.id);
  }
</script>

{#if !principal}
  <Login onUnlock={unlock} />
{:else}
  <div class="app-shell">
    <Sidebar {principal} {topics} {selectedId} {view} onHome={home} onSelect={selectTopic} onCreateTopic={() => creatingTopic = true} onSettings={settings} onLogout={logout} />
    <div class="workspace-shell">
      {#if error}
        <div class="error-banner" role="alert"><strong>Something interrupted the council.</strong><span>{error}</span><button on:click={() => error = ''}>×</button></div>
      {/if}
      {#if view === 'settings'}
        <Settings {token} />
      {:else if selectedTopic && workspace}
        <TopicRoom {token} topic={selectedTopic} {workspace} {issueTypes} onRefresh={refreshWorkspace} />
      {:else if selectedTopic && loading}
        <div class="loading-room"><span></span><span></span><span></span><p>Assembling the council…</p></div>
      {:else}
        <Overview {principal} {topics} onSelect={selectTopic} onCreate={() => creatingTopic = true} />
      {/if}
    </div>
  </div>
{/if}

{#if principal && creatingTopic}
  <CreateTopic {token} onClose={() => creatingTopic = false} onCreated={topicCreated} />
{/if}
