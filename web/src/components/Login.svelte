<script lang="ts">
  export let onUnlock: (token: string) => Promise<boolean>;

  let token = '';
  let busy = false;
  let invalid = false;

  async function submit() {
    if (!token.trim() || busy) return;
    busy = true;
    invalid = !(await onUnlock(token.trim()));
    busy = false;
  }
</script>

<main class="login-shell">
  <section class="login-story" aria-labelledby="login-title">
    <div class="brand-lockup brand-lockup--large">
      <div class="brand-mark" aria-hidden="true"><span>S</span></div>
      <span>Synod</span>
    </div>
    <p class="eyebrow">LOCAL COUNCIL · PRIVATE BY DEFAULT</p>
    <h1 id="login-title">Ideas deserve<br /><em>more than one mind.</em></h1>
    <p class="login-copy">
      Bring humans and model-backed members into one quiet room. Open an issue,
      invite a perspective, and keep every decision attached to its reasoning.
    </p>
    <div class="login-orbit" aria-hidden="true">
      <span class="orbit-seat orbit-seat--one">A</span>
      <span class="orbit-seat orbit-seat--two">R</span>
      <span class="orbit-seat orbit-seat--three">H</span>
      <span class="orbit-core">S</span>
    </div>
  </section>

  <section class="login-panel">
    <div class="login-card">
      <span class="section-kicker">ENTER THE CHAMBER</span>
      <h2>Unlock your local Synod</h2>
      <p>Use the token printed by <code>synod bootstrap</code>.</p>

      <form on:submit|preventDefault={submit}>
        <label for="token">Bearer token</label>
        <input
          id="token"
          bind:value={token}
          type="password"
          autocomplete="off"
          spellcheck="false"
          placeholder="synod_••••••••••••"
          aria-invalid={invalid}
        />
        {#if invalid}
          <p class="field-error">That token could not open this Synod.</p>
        {/if}
        <button class="button button--primary button--wide" type="submit" disabled={busy}>
          {busy ? 'Opening…' : 'Enter Synod'}
          <span aria-hidden="true">→</span>
        </button>
      </form>

      <div class="local-note">
        <span class="status-dot status-dot--ready"></span>
        <span>Only <strong>127.0.0.1</strong> · no tunnel · no cloud workspace</span>
      </div>
    </div>
  </section>
</main>
