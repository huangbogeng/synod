<script lang="ts">
  import { createIssue } from '../lib/api';
  import type { Creation, Issue, IssueType, Team, TopicMember } from '../lib/types';

  export let token: string;
  export let topicId: string;
  export let issueTypes: IssueType[];
  export let members: TopicMember[];
  export let teams: Team[];
  export let onClose: () => void;
  export let onCreated: (creation: Creation<Issue>) => void;

  let issueType = issueTypes[0]?.key ?? 'task';
  let title = '';
  let body = '';
  let busy = false;
  let error = '';

  function mention(handle: string) {
    const prefix = body && !body.endsWith(' ') && !body.endsWith('\n') ? ' ' : '';
    body += `${prefix}@${handle} `;
  }

  async function submit() {
    busy = true;
    error = '';
    try {
      onCreated(await createIssue(token, topicId, {
        issue_type: issueType,
        title,
        body,
        parent_issue_id: null
      }));
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Issue creation failed.';
    } finally {
      busy = false;
    }
  }
</script>

<div class="dialog-backdrop">
  <div class="dialog-card dialog-card--wide" role="dialog" aria-modal="true" aria-labelledby="new-issue-title" tabindex="-1">
    <header class="dialog-header"><div><span class="section-kicker">CALL THE COUNCIL</span><h2 id="new-issue-title">Open an Issue</h2></div><button class="icon-button" type="button" aria-label="Close" on:click={onClose}>×</button></header>
    <form class="stack-form" on:submit|preventDefault={submit}>
      <div class="field-pair"><label>Type<select bind:value={issueType}>{#each issueTypes as type}<option value={type.key}>{type.display_name}</option>{/each}</select></label><label>Title<input bind:value={title} required maxlength="200" placeholder="What should the council examine?" /></label></div>
      <label>Opening statement<textarea bind:value={body} rows="10" placeholder="Give enough context for another mind to respond. Mention a seat or team to start model work: @architect"></textarea></label>
      {#if members.some((member) => member.kind === 'ai') || teams.length}
        <div class="mention-picker"><span>Invite</span>{#each members.filter((member) => member.kind === 'ai') as member}<button type="button" on:click={() => mention(member.handle)}>@{member.handle}</button>{/each}{#each teams as team}<button type="button" on:click={() => mention(team.handle)}>@{team.handle}</button>{/each}</div>
      {:else}
        <p class="form-hint">This Topic has no AI seats yet. Add one from the Council panel before using a mention.</p>
      {/if}
      {#if error}<p class="form-error">{error}</p>{/if}
      <footer class="form-actions"><button class="button" type="button" on:click={onClose}>Cancel</button><button class="button button--primary" type="submit" disabled={busy}>{busy ? 'Opening…' : 'Open Issue'}</button></footer>
    </form>
  </div>
</div>
