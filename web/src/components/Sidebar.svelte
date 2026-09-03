<script lang="ts">
  import type { Principal, Topic } from '../lib/types';
  import { initial } from '../lib/presentation';

  export let principal: Principal;
  export let topics: Topic[];
  export let selectedId: string | null;
  export let onHome: () => void;
  export let onSelect: (id: string) => void;
  export let onLogout: () => void;
</script>

<aside class="sidebar">
  <button class="brand-lockup sidebar-brand" type="button" on:click={onHome}>
    <div class="brand-mark" aria-hidden="true"><span>S</span></div>
    <span>Synod</span>
  </button>

  <nav class="primary-nav" aria-label="Primary navigation">
    <button class:active={!selectedId} type="button" on:click={onHome}>
      <span class="nav-glyph">⌂</span><span>Overview</span>
    </button>
    <button type="button" disabled><span class="nav-glyph">◎</span><span>Inbox</span><small>Soon</small></button>
  </nav>

  <div class="sidebar-section">
    <div class="sidebar-heading">
      <span>TOPICS</span>
      <button type="button" aria-label="Create topic" title="Create topic is coming next">+</button>
    </div>
    <div class="topic-nav">
      {#each topics as topic}
        <button
          class:active={selectedId === topic.id}
          type="button"
          title={topic.title}
          on:click={() => onSelect(topic.id)}
        >
          <span class="topic-monogram">{initial(topic.title)}</span>
          <span class="topic-nav-copy"><strong>{topic.title}</strong><small>{topic.key}</small></span>
        </button>
      {/each}
    </div>
  </div>

  <div class="sidebar-bottom">
    <button class="settings-link" type="button" disabled><span class="nav-glyph">⚙</span><span>Settings</span></button>
    <div class="profile-chip">
      <span class="avatar avatar--human">{initial(principal.display_name)}</span>
      <span><strong>{principal.display_name}</strong><small>@{principal.handle}</small></span>
      <button type="button" title="Lock Synod" aria-label="Lock Synod" on:click={onLogout}>↗</button>
    </div>
  </div>
</aside>
