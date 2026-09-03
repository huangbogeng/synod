<script lang="ts">
  import type { Principal, Topic } from '../lib/types';
  import { excerpt, initial } from '../lib/presentation';

  export let principal: Principal;
  export let topics: Topic[];
  export let onSelect: (id: string) => void;
  export let onCreate: () => void;
</script>

<section class="overview page-enter">
  <header class="overview-header">
    <div>
      <p class="eyebrow">YOUR LOCAL COUNCIL</p>
      <h1>Good {new Date().getHours() < 12 ? 'morning' : 'afternoon'}, {principal.display_name}.</h1>
      <p>Choose a room and continue the conversation.</p>
    </div>
    <button class="button button--primary" type="button" on:click={onCreate}>
      <span>＋</span> New topic
    </button>
  </header>

  <div class="overview-signal">
    <div class="signal-icon"><span></span><span></span><span></span></div>
    <div><strong>The chamber is quiet.</strong><p>New model activity will surface here without opening your machine to the internet.</p></div>
    <span class="local-pill">LOCAL</span>
  </div>

  <div class="section-title">
    <div><span class="section-kicker">ACTIVE ROOMS</span><h2>Topics</h2></div>
    <span>{topics.length} total</span>
  </div>

  {#if topics.length}
    <div class="topic-grid">
      {#each topics as topic, index}
        <button class="topic-card" type="button" on:click={() => onSelect(topic.id)} style={`--delay:${index * 45}ms`}>
          <div class="topic-card-top">
            <span class="topic-seal">{initial(topic.title)}</span>
            <span class="topic-arrow">↗</span>
          </div>
          <small>{topic.key}</small>
          <h3>{topic.title}</h3>
          <p>{excerpt(topic.description, 135) || 'A room waiting for its first question.'}</p>
          <footer><span class="status-dot status-dot--ready"></span><span>Open chamber</span><span>Revision {topic.revision}</span></footer>
        </button>
      {/each}
    </div>
  {:else}
    <div class="empty-room">
      <span class="empty-mark">S</span>
      <h3>No topics yet</h3>
      <p>Create the first Topic and give your council a durable room.</p>
    </div>
  {/if}
</section>
