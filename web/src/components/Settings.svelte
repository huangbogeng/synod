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
  let discoveryErrors: Record<string, string> = {};

  let vendor: 'deepseek' | 'minimax' = 'deepseek';
  let providerName = 'DeepSeek';
  let credentialMode: 'api_key' | 'environment' = 'api_key';
  let apiKey = '';
  let environmentName = 'DEEPSEEK_API_KEY';
  let modelProviderId = '';
  let modelName = '';
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
        await chooseMemberProvider();
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
  }

  async function submitProvider() {
    let createdProvider: Provider | null = null;
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
      createdProvider = provider;
      modelProviderId = provider.id;
      apiKey = '';
      notice = credentialMode === 'api_key'
        ? `${providerName} is connected locally. You can now create a Member with it.`
        : `${providerName} route created. Restart Synod with ${environmentName} set before running it.`;
    });
    if (createdProvider) await discoverModels(createdProvider);
  }

  async function discoverModels(provider: Provider) {
    discovering = provider.id;
    discoveryErrors = { ...discoveryErrors, [provider.id]: '' };
    if (modelProviderId === provider.id) modelName = '';
    try {
      const models = await discoverProviderModels(token, provider.id);
      discovered = { ...discovered, [provider.id]: models };
      if (modelProviderId === provider.id) modelName = models[0]?.id ?? '';
      notice = models.length
        ? `${provider.name} connected. ${models.length} model${models.length === 1 ? '' : 's'} available.`
        : `${provider.name} connected, but it returned no available models.`;
    } catch (cause) {
      discoveryErrors = {
        ...discoveryErrors,
        [provider.id]: cause instanceof Error ? cause.message : 'Provider connection failed.'
      };
    } finally {
      discovering = '';
    }
  }

  async function chooseMemberProvider() {
    const provider = data.providers.find((candidate) => candidate.id === modelProviderId);
    if (!provider) return;
    modelName = '';
    if (Object.prototype.hasOwnProperty.call(discovered, provider.id)) {
      modelName = discovered[provider.id][0]?.id ?? '';
      return;
    }
    await discoverModels(provider);
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
          <label>Model<select bind:value={modelName} required disabled={!selectedProvider || discovering === modelProviderId || !modelChoices.length}><option value="" disabled>{discovering === modelProviderId ? 'Loading available models…' : modelChoices.length ? 'Select a model' : 'No models available'}</option>{#each modelChoices as found}<option value={found.id}>{found.id}</option>{/each}</select></label>
          {#if selectedProvider}
            <div class="member-model-tools"><button type="button" disabled={discovering === selectedProvider.id} on:click={() => discoverModels(selectedProvider)}>{discovering === selectedProvider.id ? 'Loading models…' : 'Refresh model list'}</button>{#if discoveryErrors[selectedProvider.id]}<small>{discoveryErrors[selectedProvider.id]}</small>{:else if modelChoices.length}<small>{modelChoices.length} available</small>{/if}</div>
          {/if}
          <div class="field-pair"><label>Handle<input bind:value={memberHandle} required placeholder="architect" /></label><label>Display name<input bind:value={memberDisplayName} required placeholder="Architect" /></label></div>
          <label>Identity prompt<textarea bind:value={memberPrompt} required rows="5" placeholder="Review architecture boundaries and challenge hidden assumptions."></textarea></label>
          <button class="button button--primary" type="submit" disabled={!modelName || discovering === modelProviderId || busy === 'member'}>{busy === 'member' ? 'Saving…' : 'Create member'}</button>
        </form>
        <div class="record-list">{#each data.aiMembers as member}<div><strong>{member.display_name}</strong><small>@{member.handle} · {data.models.find((model) => model.id === member.default_model_id)?.model_name ?? 'model unavailable'}</small><span>V{member.identity_prompt_version}</span></div>{:else}<p>No AI Members yet.</p>{/each}</div>
      </section>
    </div>
  {/if}
</section>
