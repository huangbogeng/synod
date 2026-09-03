<script lang="ts">
  import { createAiMember, createProvider, discoverProviderModels, loadAdminWorkspace } from '../lib/api';
  import { providerPresets } from '../lib/providerPresets';
  import type { AdminWorkspace, DiscoveredModel, Provider } from '../lib/types';

  export let token: string;

  let data: AdminWorkspace = { providers: [], models: [], aiMembers: [] };
  let loading = true;
  let busy = '';
  let error = '';
  let notice = '';
  let discovering = '';
  let discovered: Record<string, DiscoveredModel[]> = {};

  let vendor: 'deepseek' | 'minimax' = 'deepseek';
  let providerName = 'DeepSeek';
  let credentialMode: 'api_key' | 'environment' = 'api_key';
  let apiKey = '';
  let environmentName = 'DEEPSEEK_API_KEY';
  let modelProviderId = '';
  let modelName = providerPresets[0].modelName;
  let memberHandle = '';
  let memberDisplayName = '';
  let memberPrompt = '';

  $: selectedProvider = data.providers.find((provider) => provider.id === modelProviderId) ?? null;
  $: modelChoices = modelProviderId ? (discovered[modelProviderId] ?? []) : [];

  load();

  async function load() {
    loading = true;
    try {
      data = await loadAdminWorkspace(token);
      if (!modelProviderId && data.providers.length) {
        modelProviderId = data.providers[0].id;
        chooseMemberProvider();
      }
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Settings could not be loaded.';
    } finally {
      loading = false;
    }
  }

  function chooseVendor(nextVendor: 'deepseek' | 'minimax') {
    vendor = nextVendor;
    const preset = providerPresets.find((candidate) => candidate.id === vendor) ?? providerPresets[0];
    providerName = preset.name;
    environmentName = preset.environmentName;
    modelName = preset.modelName;
  }

  async function submitProvider() {
    await perform('provider', async () => {
      const preset = providerPresets.find((candidate) => candidate.id === vendor) ?? providerPresets[0];
      const provider = await createProvider(token, {
        name: providerName,
        adapter: 'openai_compatible',
        base_url: preset.baseUrl,
        ...(credentialMode === 'api_key'
          ? { api_key: apiKey }
          : { credential_ref: `env://${environmentName}` })
      });
      modelProviderId = provider.id;
      apiKey = '';
      if (!modelName) chooseVendor(vendor);
      notice = credentialMode === 'api_key'
        ? `${providerName} is connected locally. You can now create a Member with it.`
        : `${providerName} route created. Restart Synod with ${environmentName} set before running it.`;
    });
  }

  async function discoverModels(provider: Provider) {
    discovering = provider.id;
    error = '';
    notice = '';
    try {
      const models = await discoverProviderModels(token, provider.id);
      discovered = { ...discovered, [provider.id]: models };
      chooseModel(provider.id, models[0].id);
      notice = `${provider.name} connected. ${models.length} model${models.length === 1 ? '' : 's'} discovered.`;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Provider connection failed.';
    } finally {
      discovering = '';
    }
  }

  function chooseModel(providerId: string, discoveredName: string) {
    modelProviderId = providerId;
    modelName = discoveredName;
  }

  function chooseMemberProvider() {
    const provider = data.providers.find((candidate) => candidate.id === modelProviderId);
    if (!provider) return;
    const preset = providerPresets.find((candidate) => provider.base_url.startsWith(candidate.baseUrl));
    const firstDiscovered = discovered[provider.id]?.[0]?.id;
    modelName = firstDiscovered ?? preset?.modelName ?? '';
  }

  async function submitMember() {
    await perform('member', async () => {
      await createAiMember(token, {
        handle: memberHandle,
        display_name: memberDisplayName,
        identity_prompt: memberPrompt,
        provider_id: modelProviderId,
        model_name: modelName
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
  <header class="settings-header"><div><p class="eyebrow">LOCAL ADMINISTRATION</p><h1>Give the council its voices.</h1><p>Connect a Provider once. Each AI Member then chooses the model it needs and adds one identity prompt.</p></div></header>
  {#if error}<p class="settings-message settings-message--error">{error}</p>{/if}
  {#if notice}<p class="settings-message">{notice}</p>{/if}

  {#if loading}
    <div class="loading-room"><span></span><span></span><span></span><p>Reading local configuration…</p></div>
  {:else}
    <div class="settings-grid">
      <section class="settings-card">
        <header><span>01</span><div><h2>Provider route</h2><p>Choose a supported preset and keep its key local.</p></div></header>
        <form class="stack-form" on:submit|preventDefault={submitProvider}>
          <div class="preset-picker">
            {#each providerPresets as preset}
              <button class:active={vendor === preset.id} type="button" on:click={() => chooseVendor(preset.id)}><span class={`provider-orb provider-orb--${preset.accent}`}>{preset.name.charAt(0)}</span><strong>{preset.name}</strong><small>{preset.description}</small></button>
            {/each}
          </div>
          <label>Display name<input bind:value={providerName} required /></label>
          <label>Credential storage<select bind:value={credentialMode}><option value="api_key">Local API key</option><option value="environment">Environment variable</option></select></label>
          {#if credentialMode === 'api_key'}
            <label>API key<input type="password" bind:value={apiKey} required autocomplete="off" placeholder="Stored locally and never returned by the API" /></label>
          {:else}
            <label>Environment variable<input bind:value={environmentName} required pattern="[A-Z0-9_]+" /></label>
          {/if}
          <button class="button button--primary" type="submit" disabled={busy === 'provider'}>{busy === 'provider' ? 'Saving…' : 'Add provider'}</button>
        </form>
        <div class="record-list">
          {#each data.providers as provider}
            <div class="provider-record"><div><strong>{provider.name}</strong><small>{provider.base_url}</small></div><button type="button" disabled={discovering === provider.id} on:click={() => discoverModels(provider)}>{discovering === provider.id ? 'TESTING…' : 'TEST'}</button></div>
          {:else}<p>No provider routes yet.</p>{/each}
        </div>
      </section>

      <section class="settings-card">
        <header><span>02</span><div><h2>AI Member</h2><p>Choose a Provider and bind the model directly to this identity.</p></div></header>
        <form class="stack-form" on:submit|preventDefault={submitMember}>
          <label>Provider<select bind:value={modelProviderId} on:change={chooseMemberProvider} required><option value="" disabled>Select a provider</option>{#each data.providers as provider}<option value={provider.id}>{provider.name}</option>{/each}</select></label>
          <label>Model identifier<input bind:value={modelName} list="provider-models" required placeholder="Choose a discovered model or enter its exact ID" /></label>
          <datalist id="provider-models">{#each modelChoices as found}<option value={found.id}></option>{/each}</datalist>
          {#if selectedProvider}
            <div class="member-model-tools"><button type="button" disabled={discovering === selectedProvider.id} on:click={() => discoverModels(selectedProvider)}>{discovering === selectedProvider.id ? 'Testing connection…' : 'Test connection · discover models'}</button></div>
          {/if}
          {#if modelChoices.length}
            <div class="model-results">{#each modelChoices as found}<button type="button" class:active={modelName === found.id} on:click={() => chooseModel(modelProviderId, found.id)}>{found.id}</button>{/each}</div>
          {/if}
          <div class="field-pair"><label>Handle<input bind:value={memberHandle} required placeholder="architect" /></label><label>Display name<input bind:value={memberDisplayName} required placeholder="Architect" /></label></div>
          <label>Identity prompt<textarea bind:value={memberPrompt} required rows="5" placeholder="Review architecture boundaries and challenge hidden assumptions."></textarea></label>
          <button class="button button--primary" type="submit" disabled={!data.providers.length || busy === 'member'}>{busy === 'member' ? 'Saving…' : 'Create member'}</button>
        </form>
        <div class="record-list">{#each data.aiMembers as member}<div><strong>{member.display_name}</strong><small>@{member.handle} · {data.models.find((model) => model.id === member.default_model_id)?.model_name ?? 'model unavailable'}</small><span>V{member.identity_prompt_version}</span></div>{:else}<p>No AI Members yet.</p>{/each}</div>
      </section>
    </div>
  {/if}
</section>
