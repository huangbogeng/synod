<script lang="ts">
  import { createTopic } from '../lib/api';
  import type { Topic } from '../lib/types';

  export let token: string;
  export let onClose: () => void;
  export let onCreated: (topic: Topic) => void;

  let key = '';
  let title = '';
  let description = '';
  let busy = false;
  let error = '';

  function normalizeKey(value: string): string {
    return value.toLowerCase().trim().replace(/[^a-z0-9-]+/g, '-').replace(/^-+|-+$/g, '').slice(0, 32);
  }

  async function submit() {
    busy = true;
    error = '';
    try {
      const topic = await createTopic(token, { key: normalizeKey(key), title, description });
      onCreated(topic);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Topic creation failed.';
    } finally {
      busy = false;
    }
  }
</script>

<div class="dialog-backdrop">
  <div class="dialog-card" role="dialog" aria-modal="true" aria-labelledby="new-topic-title" tabindex="-1">
    <header class="dialog-header">
      <div><span class="section-kicker">OPEN A ROOM</span><h2 id="new-topic-title">Create a Topic</h2></div>
      <button class="icon-button" type="button" aria-label="Close" on:click={onClose}>×</button>
    </header>
    <form class="stack-form" on:submit|preventDefault={submit}>
      <label>Topic key<input bind:value={key} on:input={() => key = normalizeKey(key)} required minlength="2" maxlength="32" placeholder="factor-research" /></label>
      <label>Title<input bind:value={title} required maxlength="200" placeholder="Factor Research" /></label>
      <label>Description<textarea bind:value={description} rows="5" placeholder="What this long-running room is trying to learn or build."></textarea></label>
      {#if error}<p class="form-error">{error}</p>{/if}
      <footer class="form-actions"><button class="button" type="button" on:click={onClose}>Cancel</button><button class="button button--primary" type="submit" disabled={busy}>{busy ? 'Creating…' : 'Create Topic'}</button></footer>
    </form>
  </div>
</div>
