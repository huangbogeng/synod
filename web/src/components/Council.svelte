<script lang="ts">
  import type { AiMember, Model, Run, Team, TopicMember } from '../lib/types';
  import { initial } from '../lib/presentation';

  export let members: TopicMember[];
  export let teams: Team[];
  export let aiMembers: AiMember[];
  export let models: Model[];
  export let runs: Run[];

  $: aiById = new Map(aiMembers.map((member) => [member.id, member]));
  $: modelById = new Map(models.map((model) => [model.id, model]));

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

  <div class="council-footer">
    <span class="pulse-ring"><i></i></span>
    <div><strong>Runs stay observable</strong><p>Queued and active model work will stream into this panel.</p></div>
  </div>
</aside>
