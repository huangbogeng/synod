<script lang="ts">
  import { onMount } from 'svelte';
  import { ApiError, currentPrincipal, listTopics, loadTopicWorkspace } from './lib/api';
  import type { Principal, Topic, TopicWorkspace } from './lib/types';
  import Login from './components/Login.svelte';
  import Overview from './components/Overview.svelte';
  import Sidebar from './components/Sidebar.svelte';
  import TopicRoom from './components/TopicRoom.svelte';

  const SESSION_KEY = 'synod.session.token';
  let token = '';
  let principal: Principal | null = null;
  let topics: Topic[] = [];
  let selectedId: string | null = null;
  let workspace: TopicWorkspace | null = null;
  let loading = false;
  let error = '';

  $: selectedTopic = topics.find((topic) => topic.id === selectedId) ?? null;

  onMount(async () => {
    const saved = sessionStorage.getItem(SESSION_KEY);
    if (saved) await unlock(saved);
  });

  async function unlock(candidate: string): Promise<boolean> {
    loading = true;
    error = '';
    try {
      const [nextPrincipal, nextTopics] = await Promise.all([
        currentPrincipal(candidate),
        listTopics(candidate)
      ]);
      token = candidate;
      principal = nextPrincipal;
      topics = nextTopics;
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
    if (selectedId === id && workspace) return;
    selectedId = id;
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

  function home() {
    selectedId = null;
    workspace = null;
    error = '';
  }

  function logout() {
    sessionStorage.removeItem(SESSION_KEY);
    token = '';
    principal = null;
    topics = [];
    home();
  }
</script>

{#if !principal}
  <Login onUnlock={unlock} />
{:else}
  <div class="app-shell">
    <Sidebar {principal} {topics} {selectedId} onHome={home} onSelect={selectTopic} onLogout={logout} />
    <div class="workspace-shell">
      {#if error}
        <div class="error-banner" role="alert"><strong>Something interrupted the council.</strong><span>{error}</span><button on:click={() => error = ''}>×</button></div>
      {/if}
      {#if selectedTopic && workspace}
        <TopicRoom topic={selectedTopic} {workspace} />
      {:else if selectedTopic && loading}
        <div class="loading-room"><span></span><span></span><span></span><p>Assembling the council…</p></div>
      {:else}
        <Overview {principal} {topics} onSelect={selectTopic} />
      {/if}
    </div>
  </div>
{/if}
