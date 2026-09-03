<script lang="ts">
  import { createAiMember, createModel, createProvider, loadAdminWorkspace } from '../lib/api';
  import type { AdminWorkspace } from '../lib/types';

  export let token: string;

  let data: AdminWorkspace = { providers: [], models: [], aiMembers: [] };
  let loading = true;
  let busy = '';
  let error = '';
  let notice = '';

  let vendor: 'deepseek' | 'minimax' = 'deepseek';
  let providerName = 'DeepSeek';
  let environmentName = 'DEEPSEEK_API_KEY';
  let modelProviderId = '';
  let modelName = '';
  let modelDisplayName = '';
  let memberModelId = '';
  let memberHandle = '';
  let memberDisplayName = '';
  let memberPrompt = '';

  $: if (!modelProviderId && data.providers.length) modelProviderId = data.providers[0].id;
  $: if (!memberModelId && data.models.length) memberModelId = data.models[0].id;

  load();

  async function load() {
    loading = true;
    try {
      data = await loadAdminWorkspace(token);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Settings could not be loaded.';
    } finally {
      loading = false;
    }
  }

  function chooseVendor() {
    providerName = vendor === 'deepseek' ? 'DeepSeek' : 'MiniMax';
    environmentName = vendor === 'deepseek' ? 'DEEPSEEK_API_KEY' : 'MINIMAX_API_KEY';
  }

  async function submitProvider() {
    await perform('provider', async () => {
      await createProvider(token, {
        name: providerName,
        adapter: 'openai_compatible',
        base_url: vendor === 'deepseek' ? 'https://api.deepseek.com' : 'https://api.minimaxi.com/v1',
        credential_ref: `env://${environmentName}`
      });
      notice = `${providerName} route created. Restart Synod with ${environmentName} set before running it.`;
    });
  }

  async function submitModel() {
    await perform('model', async () => {
      await createModel(token, {
        provider_id: modelProviderId,
        model_name: modelName,
        display_name: modelDisplayName,
        capabilities: { streaming: false, tool_calling: false }
      });
      modelName = '';
      modelDisplayName = '';
      notice = 'Model added. You can now give an AI Member this voice.';
    });
  }

  async function submitMember() {
    await perform('member', async () => {
      await createAiMember(token, {
        handle: memberHandle,
        display_name: memberDisplayName,
        identity_prompt: memberPrompt,
        default_model_id: memberModelId
      });
      memberHandle = '';
      memberDisplayName = '';
      memberPrompt = '';
      notice = 'AI Member created. Add it to a Topic Council before mentioning it.';
    });
  }

  async function perform(kind: string, operation: () => Promise<void>) {
    busy = kind;
    error = '';
    notice = '';
    try {
      await operation();
      data = await loadAdminWorkspace(token);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'The change could not be saved.';
    } finally {
      busy = '';
    }
  }
</script>

<section class="settings-page page-enter">
  <header class="settings-header"><div><p class="eyebrow">LOCAL ADMINISTRATION</p><h1>Give the council its voices.</h1><p>Connect a provider, register a model, then shape an AI Member with one identity prompt.</p></div></header>
  {#if error}<p class="settings-message settings-message--error">{error}</p>{/if}
  {#if notice}<p class="settings-message">{notice}</p>{/if}

  {#if loading}
    <div class="loading-room"><span></span><span></span><span></span><p>Reading local configuration…</p></div>
  {:else}
    <div class="settings-grid">
      <section class="settings-card">
        <header><span>01</span><div><h2>Provider route</h2><p>The secret stays in an environment variable.</p></div></header>
        <form class="stack-form" on:submit|preventDefault={submitProvider}>
          <label>Vendor<select bind:value={vendor} on:change={chooseVendor}><option value="deepseek">DeepSeek</option><option value="minimax">MiniMax</option></select></label>
          <label>Display name<input bind:value={providerName} required /></label>
          <label>Environment variable<input bind:value={environmentName} required pattern="[A-Z0-9_]+" /></label>
          <button class="button button--primary" type="submit" disabled={busy === 'provider'}>{busy === 'provider' ? 'Saving…' : 'Add provider'}</button>
        </form>
        <div class="record-list">{#each data.providers as provider}<div><strong>{provider.name}</strong><small>{provider.base_url}</small><span>{provider.enabled ? 'READY' : 'OFF'}</span></div>{:else}<p>No provider routes yet.</p>{/each}</div>
      </section>

      <section class="settings-card">
        <header><span>02</span><div><h2>Model</h2><p>Use the exact model identifier from the vendor.</p></div></header>
        <form class="stack-form" on:submit|preventDefault={submitModel}>
          <label>Provider<select bind:value={modelProviderId} required><option value="" disabled>Select a provider</option>{#each data.providers as provider}<option value={provider.id}>{provider.name}</option>{/each}</select></label>
          <label>Model identifier<input bind:value={modelName} required placeholder="deepseek-chat" /></label>
          <label>Display name<input bind:value={modelDisplayName} required placeholder="DeepSeek Reviewer" /></label>
          <button class="button button--primary" type="submit" disabled={!data.providers.length || busy === 'model'}>{busy === 'model' ? 'Saving…' : 'Add model'}</button>
        </form>
        <div class="record-list">{#each data.models as model}<div><strong>{model.display_name}</strong><small>{model.model_name}</small><span>{model.enabled ? 'READY' : 'OFF'}</span></div>{:else}<p>No models yet.</p>{/each}</div>
      </section>

      <section class="settings-card">
        <header><span>03</span><div><h2>AI Member</h2><p>Identity is a prompt; model routing stays separate.</p></div></header>
        <form class="stack-form" on:submit|preventDefault={submitMember}>
          <label>Default model<select bind:value={memberModelId} required><option value="" disabled>Select a model</option>{#each data.models as model}<option value={model.id}>{model.display_name}</option>{/each}</select></label>
          <div class="field-pair"><label>Handle<input bind:value={memberHandle} required placeholder="architect" /></label><label>Display name<input bind:value={memberDisplayName} required placeholder="Architect" /></label></div>
          <label>Identity prompt<textarea bind:value={memberPrompt} required rows="5" placeholder="Review architecture boundaries and challenge hidden assumptions."></textarea></label>
          <button class="button button--primary" type="submit" disabled={!data.models.length || busy === 'member'}>{busy === 'member' ? 'Saving…' : 'Create member'}</button>
        </form>
        <div class="record-list">{#each data.aiMembers as member}<div><strong>{member.display_name}</strong><small>@{member.handle}</small><span>V{member.identity_prompt_version}</span></div>{:else}<p>No AI Members yet.</p>{/each}</div>
      </section>
    </div>
  {/if}
</section>
