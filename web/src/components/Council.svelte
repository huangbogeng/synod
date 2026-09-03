<script lang="ts">
  import type { AiMember, Model, Run, Team, TopicMember } from '../lib/types';
  import { initial } from '../lib/presentation';

  export let members: TopicMember[];
  export let teams: Team[];
  export let aiMembers: AiMember[];
  export let models: Model[];
  export let runs: Run[];
  export let onAddMember: (memberId: string) => Promise<void>;
  export let onCreateTeam: (input: { handle: string; displayName: string; memberId: string }) => Promise<void>;

  let selectedMember = '';
  let teamMember = '';
  let teamHandle = '';
  let teamName = '';
  let managing = false;
  let message = '';

  $: aiById = new Map(aiMembers.map((member) => [member.id, member]));
  $: modelById = new Map(models.map((model) => [model.id, model]));
  $: available = aiMembers.filter((member) => !members.some((seat) => seat.id === member.id));
  $: if (!selectedMember && available.length) selectedMember = available[0].id;
  $: if (!teamMember && aiMembers.length) teamMember = aiMembers[0].id;

  async function addSeat() {
    if (!selectedMember) return;
    managing = true;
    message = '';
    try {
      await onAddMember(selectedMember);
      selectedMember = '';
      message = 'Seat added.';
    } catch (cause) {
      message = cause instanceof Error ? cause.message : 'Seat could not be added.';
    } finally {
      managing = false;
    }
  }

  async function addTeam() {
    managing = true;
    message = '';
    try {
      await onCreateTeam({ handle: teamHandle, displayName: teamName, memberId: teamMember });
      teamHandle = '';
      teamName = '';
      message = 'Team added.';
    } catch (cause) {
      message = cause instanceof Error ? cause.message : 'Team could not be added.';
    } finally {
      managing = false;
    }
  }

  function subtitle(member: TopicMember): string {
    const ai = aiById.get(member.id);
    if (!ai) return member.kind === 'human' ? 'Human chair' : member.role;
    return modelById.get(ai.default_model_id)?.display_name ?? 'Model unavailable';
  }

  function state(member: TopicMember): string {
    const related = runs.filter((run) => run.ai_member_id === member.id);
    if (related.some((run) => run.status === 'in_progress')) return 'SPEAKING';
    if (related.some((run) => run.status === 'queued')) return 'QUEUED';
    return member.kind === 'ai' ? 'READY' : member.role.toUpperCase();
  }
</script>

<aside class="council-panel">
  <header class="council-header">
    <div><span class="section-kicker">PRESENT</span><h2>Council</h2></div>
    <span class="count-badge">{members.length}</span>
  </header>
  <p class="council-intro">Every voice has a seat. Mention a member or team inside an Issue to call them in.</p>

  <div class="seat-list">
    {#each members as member}
      <button class="council-seat" type="button">
        <span class:avatar--ai={member.kind === 'ai'} class:avatar--human={member.kind === 'human'} class="avatar">
          {initial(member.display_name)}
          <i></i>
        </span>
        <span class="seat-copy"><strong>{member.display_name}</strong><small>{subtitle(member)}</small></span>
        <span class:seat-state--active={state(member) === 'SPEAKING'} class="seat-state">{state(member)}</span>
      </button>
    {:else}
      <div class="council-empty">This Topic has no visible members.</div>
    {/each}
  </div>

  {#if teams.length}
    <div class="teams-block">
      <span class="section-kicker">TEAMS</span>
      {#each teams as team}
        <div class="team-row"><span>@{team.handle}</span><small>{team.members.length} seats</small></div>
      {/each}
    </div>
  {/if}

  <details class="council-manage">
    <summary>Manage council</summary>
    {#if aiMembers.length}
      <form on:submit|preventDefault={addSeat}>
        <label>Add an AI seat<select bind:value={selectedMember} disabled={!available.length}>{#each available as member}<option value={member.id}>@{member.handle}</option>{/each}{#if !available.length}<option>All members seated</option>{/if}</select></label>
        <button class="button" type="submit" disabled={!available.length || managing}>Add</button>
      </form>
      <form class="team-form" on:submit|preventDefault={addTeam}>
        <label>Team handle<input bind:value={teamHandle} required placeholder="reviewers" /></label>
        <label>Team name<input bind:value={teamName} required placeholder="Review team" /></label>
        <label>First seat<select bind:value={teamMember}>{#each aiMembers as member}<option value={member.id}>@{member.handle}</option>{/each}</select></label>
        <button class="button" type="submit" disabled={managing}>Create team</button>
      </form>
      {#if message}<p>{message}</p>{/if}
    {:else}
      <p>Create an AI Member in Settings first.</p>
    {/if}
  </details>

  <div class="council-footer">
    <span class="pulse-ring"><i></i></span>
    <div><strong>Runs stay observable</strong><p>Queued and active model work will stream into this panel.</p></div>
  </div>
</aside>
