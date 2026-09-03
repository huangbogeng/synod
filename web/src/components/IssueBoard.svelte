<script lang="ts">
  import type { Issue, Run } from '../lib/types';
  import { excerpt, issueLabel } from '../lib/presentation';

  export let issues: Issue[];
  export let runs: Run[];

  $: open = issues.filter((issue) => laneFor(issue) === 'open');
  $: discussing = issues.filter((issue) => laneFor(issue) === 'discussing');
  $: decisions = issues.filter((issue) => laneFor(issue) === 'decision');
  $: closed = issues.filter((issue) => issue.state === 'closed');

  function issueRuns(issue: Issue): Run[] {
    return runs.filter((run) => run.item_id === issue.id);
  }

  function laneFor(issue: Issue): 'open' | 'discussing' | 'decision' | 'closed' {
    if (issue.state === 'closed') return 'closed';
    const related = issueRuns(issue);
    if (related.some((run) => run.status === 'queued' || run.status === 'in_progress')) return 'discussing';
    if (related.some((run) => run.status === 'completed') || issue.issue_type === 'decision') return 'decision';
    return 'open';
  }

  const lanes = [
    { key: 'open', title: 'Open floor', note: 'Ready for a voice' },
    { key: 'discussing', title: 'In council', note: 'Model runs appear here' },
    { key: 'decision', title: 'Decision needed', note: 'A human call remains' },
    { key: 'closed', title: 'Resolved', note: 'Kept for the record' }
  ] as const;

  function laneIssues(key: typeof lanes[number]['key']): Issue[] {
    if (key === 'open') return open;
    if (key === 'discussing') return discussing;
    if (key === 'decision') return decisions;
    if (key === 'closed') return closed;
    return [];
  }
</script>

<div class="board" aria-label="Issue board">
  {#each lanes as lane}
    <section class={`lane lane--${lane.key}`}>
      <header>
        <div><span class="lane-dot"></span><h3>{lane.title}</h3><strong>{laneIssues(lane.key).length}</strong></div>
        <button type="button" aria-label={`Add to ${lane.title}`} disabled>＋</button>
      </header>
      <p class="lane-note">{lane.note}</p>
      <div class="lane-stack">
        {#each laneIssues(lane.key) as issue}
          <article class="issue-card">
            <div class="issue-meta">
              <span class={`issue-type issue-type--${issue.issue_type}`}>{issueLabel(issue.issue_type)}</span>
              <span>#{issue.number}</span>
            </div>
            <h4>{issue.title}</h4>
            <p>{excerpt(issue.body) || 'No description was added.'}</p>
            <footer>
              <span class="mini-avatar">{issue.issue_type.charAt(0).toUpperCase()}</span>
              {#if issue.parent_issue_id}<span>↳ child issue</span>{:else}<span>Open issue</span>{/if}
              {#if issueRuns(issue).length}<span>{issueRuns(issue).length} run{issueRuns(issue).length === 1 ? '' : 's'}</span>{/if}
              <span class="issue-open">↗</span>
            </footer>
          </article>
        {:else}
          <div class="lane-empty"><span>·</span><p>Nothing here</p></div>
        {/each}
      </div>
    </section>
  {/each}
</div>
