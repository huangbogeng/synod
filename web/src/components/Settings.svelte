<script lang="ts">
  import { createAiMember, createProvider, discoverProviderModels, loadAdminWorkspace } from '../lib/api';
  import { providerPresets } from '../lib/providerPresets';
  import type { AdminWorkspace, AiMember, DiscoveredModel, Model, Provider } from '../lib/types';

  export let token: string;

  const memberTemplates = [
    { name: 'Architect', prompt: 'Review architecture boundaries, scalability, migration risk, and unnecessary complexity. Challenge hidden assumptions and propose concrete alternatives.' },
    { name: 'Critic', prompt: 'Act as a rigorous critic. Look for contradictions, missing evidence, edge cases, and reasons the proposal may fail before suggesting improvements.' },
    { name: 'Security', prompt: 'Review trust boundaries, permissions, secret handling, injection risks, abuse cases, and failure recovery. Be precise about severity and mitigations.' },
    { name: 'Researcher', prompt: 'Investigate the question methodically. Separate evidence from inference, compare plausible explanations, and identify the next experiment that would reduce uncertainty.' }
  ];

  let data: AdminWorkspace = { providers: [], models: [], aiMembers: [] };
  let loading = true;
  let busy = '';
  let error = '';
  let notice = '';
  let tab: 'providers' | 'members' = 'providers';
  let providerDialog = false;
  let memberDialog = false;
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
  let memberTemperature = 0.6;

  $: selectedProvider = data.providers.find((provider) => provider.id === modelProviderId) ?? null;
  $: modelChoices = modelProviderId ? (discovered[modelProviderId] ?? []) : [];

  load();

  async function load() {
    loading = true;
    try {
      data = await loadAdminWorkspace(token);
      if (!modelProviderId && data.providers.length) modelProviderId = data.providers[0].id;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Settings could not be loaded.';
    } finally {
      loading = false;
    }
  }

  function openProviderDialog() {
    error = '';
    vendor = 'deepseek';
    providerName = 'DeepSeek';
    credentialMode = 'api_key';
    apiKey = '';
    environmentName = 'DEEPSEEK_API_KEY';
    providerDialog = true;
  }

  async function openMemberDialog() {
    error = '';
    memberHandle = '';
    memberDisplayName = '';
    memberPrompt = '';
    memberTemperature = 0.6;
    modelProviderId = data.providers[0]?.id ?? '';
    modelName = '';
    memberDialog = true;
    if (modelProviderId) await chooseMemberProvider();
  }

  function closeDialogs() {
    if (busy) return;
    providerDialog = false;
    memberDialog = false;
    error = '';
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') closeDialogs();
  }

  function chooseVendor(nextVendor: 'deepseek' | 'minimax') {
    vendor = nextVendor;
    const preset = providerPresets.find((candidate) => candidate.id === vendor) ?? providerPresets[0];
    providerName = preset.name;
    environmentName = preset.environmentName;
  }

  async function submitProvider() {
    const saved = await perform('provider', async () => {
      const preset = providerPresets.find((candidate) => candidate.id === vendor) ?? providerPresets[0];
      await createProvider(token, {
        name: providerName,
        adapter: 'openai_compatible',
        base_url: preset.baseUrl,
        ...(credentialMode === 'api_key'
          ? { api_key: apiKey }
          : { credential_ref: `env://${environmentName}` })
      });
    });
    if (!saved) return;
    providerDialog = false;
    notice = `${providerName} was added to Provider routes.`;
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
        ? `${provider.name} responded with ${models.length} available model${models.length === 1 ? '' : 's'}.`
        : `${provider.name} connected, but returned no available models.`;
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : 'Provider connection failed.';
      discoveryErrors = { ...discoveryErrors, [provider.id]: message };
      if (!memberDialog) error = message;
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

  function applyTemplate(prompt: string) {
    memberPrompt = prompt;
  }

  async function submitMember() {
    const saved = await perform('member', async () => {
      await createAiMember(token, {
        handle: memberHandle,
        display_name: memberDisplayName,
        identity_prompt: memberPrompt,
        provider_id: modelProviderId,
        model_name: modelName,
        execution_defaults: { temperature: memberTemperature }
      });
    });
    if (!saved) return;
    memberDialog = false;
    notice = `${memberDisplayName} joined the Member registry.`;
  }

  async function perform(kind: string, operation: () => Promise<void>): Promise<boolean> {
    busy = kind;
    error = '';
    notice = '';
    try {
      await operation();
      data = await loadAdminWorkspace(token);
      return true;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'The change could not be saved.';
      return false;
    } finally {
      busy = '';
    }
  }

  function providerVendor(provider: Provider): string {
    return provider.base_url.includes('deepseek') ? 'DeepSeek' : provider.base_url.includes('minimax') ? 'MiniMax' : 'Provider';
  }

  function providerAccent(provider: Provider): string {
    return provider.base_url.includes('deepseek') ? 'deepseek' : 'minimax';
  }

  function providerHost(provider: Provider): string {
    try { return new URL(provider.base_url).host; } catch { return provider.base_url; }
  }

  function modelFor(member: AiMember): Model | null {
    return data.models.find((model) => model.id === member.default_model_id) ?? null;
  }

  function providerFor(member: AiMember): Provider | null {
    const model = modelFor(member);
    return model ? data.providers.find((provider) => provider.id === model.provider_id) ?? null : null;
  }

  function initials(value: string): string {
    const words = value.trim().split(/\s+/).filter(Boolean);
    return (words.length > 1 ? `${words[0][0]}${words[1][0]}` : value.slice(0, 2)).toUpperCase();
  }

  function memberHue(handle: string): number {
    return [...handle].reduce((value, character) => value + character.charCodeAt(0) * 17, 0) % 360;
  }

  function memberTemperatureFor(member: AiMember): string {
    const value = member.execution_defaults.temperature;
    return typeof value === 'number' ? value.toFixed(1) : 'default';
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<section class="settings-page page-enter">
  <header class="settings-header"><div><p class="eyebrow">LOCAL ADMINISTRATION</p><h1>Shape the council.</h1><p>Manage the routes that carry model traffic and the distinct voices that join each Topic.</p></div></header>

  <nav class="settings-tabs" aria-label="Settings sections">
    <button class:active={tab === 'providers'} type="button" on:click={() => tab = 'providers'}>Providers <span>{data.providers.length}</span></button>
    <button class:active={tab === 'members'} type="button" on:click={() => tab = 'members'}>AI Members <span>{data.aiMembers.length}</span></button>
  </nav>

  {#if error}<p class="settings-message settings-message--error">{error}</p>{/if}
  {#if notice}<p class="settings-message">{notice}</p>{/if}

  {#if loading}
    <div class="loading-room"><span></span><span></span><span></span><p>Reading local configuration…</p></div>
  {:else if tab === 'providers'}
    <section class="resource-page page-enter">
      <header class="resource-toolbar"><div><p class="section-kicker">ROUTES</p><h2>Provider instances</h2><p>Each card is one credentialed route. Members can use different routes at the same time.</p></div><button class="button button--primary" type="button" on:click={openProviderDialog}><span>＋</span> New Provider</button></header>
      <div class="provider-gallery">
        {#each data.providers as provider}
          <article class="provider-tile">
            <div class={`provider-emblem provider-emblem--${providerAccent(provider)}`}>{provider.name.charAt(0).toUpperCase()}</div>
            <div class="provider-tile-copy"><div><h3>{provider.name}</h3><span class="resource-badge">{providerVendor(provider)}</span></div><p>{providerHost(provider)}</p><footer><span class="credential-state" class:missing={!provider.credential_configured}><i></i>{provider.credential_configured ? 'Credential configured' : 'Credential missing'}</span><button type="button" disabled={discovering === provider.id} on:click={() => discoverModels(provider)}>{discovering === provider.id ? 'Testing…' : 'Test route'}</button></footer></div>
          </article>
        {/each}
        <button class="resource-add-card" type="button" on:click={openProviderDialog}><span>＋</span><strong>New Provider</strong><small>Add a credentialed model route</small></button>
      </div>
    </section>
  {:else}
    <section class="resource-page member-page page-enter">
      <header class="resource-toolbar"><div><p class="section-kicker">COUNCIL ROSTER</p><h2>A hall of distinct voices</h2><p>Identity comes from the mandate. Provider and model determine how that voice is carried.</p></div><button class="button button--primary" type="button" on:click={openMemberDialog}><span>＋</span> Invite a voice</button></header>
      <div class="member-gallery">
        {#each data.aiMembers as member}
          <article class="member-card" style={`--member-hue:${memberHue(member.handle)}deg`}>
            <div class="member-card-pattern"></div><div class="member-sigil"><span>{initials(member.display_name)}</span><i></i></div>
            <div class="member-card-copy"><p class="member-label">AI MEMBER · PROMPT V{member.identity_prompt_version} · TEMP {memberTemperatureFor(member)}</p><h3>{member.display_name}</h3><strong>@{member.handle}</strong><div class="member-mandate">{member.identity_prompt}</div></div>
            <footer><span>{providerFor(member)?.name ?? 'Provider unavailable'}</span><i>→</i><span>{modelFor(member)?.model_name ?? 'Model unavailable'}</span></footer>
          </article>
        {/each}
        <button class="resource-add-card resource-add-card--member" type="button" on:click={openMemberDialog}><span>＋</span><strong>Invite a member</strong><small>Give the council another point of view</small></button>
      </div>
    </section>
  {/if}
</section>

{#if providerDialog}
  <div class="dialog-backdrop"><button class="dialog-scrim" type="button" aria-label="Close Provider dialog" on:click={closeDialogs}></button><div class="dialog-card provider-dialog" role="dialog" aria-modal="true" aria-labelledby="provider-dialog-title">
    <header class="dialog-header"><div><p class="eyebrow">NEW ROUTE</p><h2 id="provider-dialog-title">Add a Provider</h2><p>Start with a supported vendor. Technical defaults stay out of the way.</p></div><button class="icon-button" type="button" aria-label="Close" on:click={closeDialogs}>×</button></header>
    <form class="stack-form" on:submit|preventDefault={submitProvider}>
      {#if error}<p class="form-error">{error}</p>{/if}
      <div class="preset-picker">
        {#each providerPresets as preset}
          <button class:active={vendor === preset.id} type="button" on:click={() => chooseVendor(preset.id)}><span class={`provider-orb provider-orb--${preset.accent}`}>{preset.name.charAt(0)}</span><strong>{preset.name}</strong><small>{preset.description}</small></button>
        {/each}
      </div>
      <label>Instance name<input bind:value={providerName} required placeholder="DeepSeek Main" /></label>
      <label>Credential storage<select bind:value={credentialMode}><option value="api_key">Local API key</option><option value="environment">Environment variable</option></select></label>
      {#if credentialMode === 'api_key'}
        <label>API key<input type="password" bind:value={apiKey} required autocomplete="off" placeholder="Stored locally and never returned by the API" /></label>
      {:else}
        <label>Environment variable<input bind:value={environmentName} required pattern="[A-Z0-9_]+" /></label>
      {/if}
      <div class="form-actions"><button class="button" type="button" on:click={closeDialogs}>Cancel</button><button class="button button--primary" type="submit" disabled={busy === 'provider'}>{busy === 'provider' ? 'Adding…' : 'Add Provider'}</button></div>
    </form>
  </div></div>
{/if}

{#if memberDialog}
  <div class="dialog-backdrop"><button class="dialog-scrim" type="button" aria-label="Close Member dialog" on:click={closeDialogs}></button><div class="dialog-card member-dialog" role="dialog" aria-modal="true" aria-labelledby="member-dialog-title">
    <header class="dialog-header"><div><p class="eyebrow">INVITE A VOICE</p><h2 id="member-dialog-title">Create an AI Member</h2><p>One identity mandate, carried by one Provider and model.</p></div><button class="icon-button" type="button" aria-label="Close" on:click={closeDialogs}>×</button></header>
    {#if !data.providers.length}
      <div class="member-no-provider"><div class="member-sigil member-sigil--empty">＋</div><h3>A voice needs a route first.</h3><p>Add a Provider before inviting an AI Member.</p><button class="button button--primary" type="button" on:click={() => { memberDialog = false; tab = 'providers'; openProviderDialog(); }}>Add Provider</button></div>
    {:else}
      <div class="member-builder">
        <form class="stack-form" on:submit|preventDefault={submitMember}>
          {#if error}<p class="form-error">{error}</p>{/if}
          <div class="field-pair"><label>Display name<input bind:value={memberDisplayName} required placeholder="Architect" /></label><label>Handle<input bind:value={memberHandle} required placeholder="architect" pattern="[a-z0-9][a-z0-9-]*" /></label></div>
          <fieldset class="mandate-field"><legend>Mandate</legend><div class="template-picker">{#each memberTemplates as template}<button class:active={memberPrompt === template.prompt} type="button" on:click={() => applyTemplate(template.prompt)}>{template.name}</button>{/each}</div><textarea bind:value={memberPrompt} required rows="7" placeholder="Describe the point of view this Member should consistently bring to the council."></textarea></fieldset>
          <div class="field-pair"><label>Provider<select bind:value={modelProviderId} on:change={chooseMemberProvider} required>{#each data.providers as provider}<option value={provider.id}>{provider.name}</option>{/each}</select></label><label>Model<select bind:value={modelName} required disabled={discovering === modelProviderId || !modelChoices.length}><option value="" disabled>{discovering === modelProviderId ? 'Loading models…' : modelChoices.length ? 'Select a model' : 'No models available'}</option>{#each modelChoices as found}<option value={found.id}>{found.id}</option>{/each}</select></label></div>
          <label>Temperature · {memberTemperature.toFixed(1)}<input bind:value={memberTemperature} type="range" min="0" max="1" step="0.1" /></label>
          {#if selectedProvider}<div class="member-model-tools"><button type="button" disabled={discovering === selectedProvider.id} on:click={() => discoverModels(selectedProvider)}>{discovering === selectedProvider.id ? 'Requesting model list…' : 'Refresh model list'}</button>{#if discoveryErrors[selectedProvider.id]}<small>{discoveryErrors[selectedProvider.id]}</small>{:else if modelChoices.length}<small>{modelChoices.length} models available</small>{/if}</div>{/if}
          <div class="form-actions"><button class="button" type="button" on:click={closeDialogs}>Cancel</button><button class="button button--primary" type="submit" disabled={!modelName || discovering === modelProviderId || busy === 'member'}>{busy === 'member' ? 'Inviting…' : 'Invite Member'}</button></div>
        </form>
        <aside class="member-preview"><p class="section-kicker">LIVE PREVIEW</p><article class="member-card member-card--preview" style={`--member-hue:${memberHue(memberHandle || 'new-member')}deg`}><div class="member-card-pattern"></div><div class="member-sigil"><span>{initials(memberDisplayName || 'New')}</span><i></i></div><div class="member-card-copy"><p class="member-label">AI MEMBER · NEW VOICE · TEMP {memberTemperature.toFixed(1)}</p><h3>{memberDisplayName || 'Unnamed Member'}</h3><strong>@{memberHandle || 'handle'}</strong><div class="member-mandate">{memberPrompt || 'Their mandate will appear here as you shape this voice.'}</div></div><footer><span>{selectedProvider?.name ?? 'Choose Provider'}</span><i>→</i><span>{modelName || 'Choose model'}</span></footer></article></aside>
      </div>
    {/if}
  </div></div>
{/if}
