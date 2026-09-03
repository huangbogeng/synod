<script lang="ts">
  import { onMount } from 'svelte';
  import { createComment, listComments } from '../lib/api';
  import { issueLabel } from '../lib/presentation';
  import type { AiMember, Comment, CommentKind, Issue, Run, Team, TopicMember } from '../lib/types';

  export let token: string;
  export let issue: Issue;
  export let members: TopicMember[];
  export let aiMembers: AiMember[];
  export let teams: Team[];
  export let runs: Run[];
  export let onBack: () => void;
  export let onRefresh: () => Promise<void>;

  let comments: Comment[] = [];
  let loadedIssue = '';
  let kind: CommentKind = 'discussion';
  let body = '';
  let busy = false;
  let error = '';

  $: relatedRuns = runs.filter((run) => run.item_id === issue.id);
  $: if (issue.id !== loadedIssue) load();

  onMount(() => {
    const timer = window.setInterval(async () => {
      if (relatedRuns.some((run) => run.status !== 'completed')) await refreshAll();
    }, 2200);
    return () => window.clearInterval(timer);
  });

  async function load() {
    loadedIssue = issue.id;
    try {
      comments = await listComments(token, issue.id);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Comments could not be loaded.';
    }
  }

  async function refreshAll() {
    await Promise.all([load(), onRefresh()]);
  }

  function author(id: string): { name: string; handle: string; ai: boolean } {
    const member = members.find((candidate) => candidate.id === id) ?? aiMembers.find((candidate) => candidate.id === id);
    return member ? { name: member.display_name, handle: member.handle, ai: member.kind === 'ai' } : { name: 'Unknown member', handle: 'unknown', ai: false };
  }

  function mention(handle: string) {
    const prefix = body && !body.endsWith(' ') && !body.endsWith('\n') ? ' ' : '';
    body += `${prefix}@${handle} `;
  }

  async function submit() {
    busy = true;
    error = '';
    try {
      await createComment(token, issue.id, { kind, body, reply_to_comment_id: null });
      body = '';
      kind = 'discussion';
      await refreshAll();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Comment could not be posted.';
    } finally {
      busy = false;
    }
  }
</script>

<section class="issue-detail page-enter">
  <header class="issue-detail-header">
    <button class="back-button" type="button" on:click={onBack}>← Board</button>
    <div class="issue-meta"><span class={`issue-type issue-type--${issue.issue_type}`}>{issueLabel(issue.issue_type)}</span><span>#{issue.number}</span><span>{issue.state}</span></div>
    <h2>{issue.title}</h2>
  </header>

  <div class="timeline">
    <article class="timeline-entry timeline-entry--opening">
      <span class="avatar avatar--human">H</span>
      <div><header><strong>Opening statement</strong><small>revision {issue.revision}</small></header><p class="prose">{issue.body || 'No description was added.'}</p></div>
    </article>

    {#each comments as comment}
      {@const voice = author(comment.author_id)}
      <article class:timeline-entry--ai={voice.ai} class="timeline-entry">
        <span class:avatar--ai={voice.ai} class:avatar--human={!voice.ai} class="avatar">{voice.name.charAt(0).toUpperCase()}</span>
        <div><header><strong>{voice.name}</strong><span>@{voice.handle}</span><small>{comment.kind}</small></header><p class="prose">{comment.body}</p></div>
      </article>
    {/each}

    {#if relatedRuns.length}
      <div class="run-strip">{#each relatedRuns as run}<span class:run-chip--active={run.status !== 'completed'} class:run-chip--failed={run.conclusion === 'failure'} class="run-chip">{run.status === 'completed' ? (run.conclusion ?? 'completed') : run.status.replace('_', ' ')}</span>{/each}</div>
    {/if}
  </div>

  <form class="comment-composer" on:submit|preventDefault={submit}>
    <div class="composer-top"><select bind:value={kind}><option value="discussion">Discussion</option><option value="direction">Human direction</option><option value="evidence">Evidence</option><option value="progress">Progress</option><option value="result">Result</option></select><span>Use a mention to call another voice</span></div>
    <textarea bind:value={body} required rows="5" placeholder="Continue the thread, or write @architect to ask for another pass."></textarea>
    <footer><div class="mention-picker">{#each members.filter((member) => member.kind === 'ai') as member}<button type="button" on:click={() => mention(member.handle)}>@{member.handle}</button>{/each}{#each teams as team}<button type="button" on:click={() => mention(team.handle)}>@{team.handle}</button>{/each}</div><button class="button button--primary" type="submit" disabled={busy}>{busy ? 'Posting…' : 'Post comment'}</button></footer>
    {#if error}<p class="form-error">{error}</p>{/if}
  </form>
</section>
