# Dispatches and Runs

## Principle

A mention event and a model execution are different things.

```text
source event
  -> Dispatch
       -> zero or more human notifications
       -> one Run per expanded AI Member
```

This allows each AI Member to fail, finish, cancel, retry, and report usage
independently.

## Dispatch

A Dispatch is created from one Issue body, Proposal body, Comment, direct reply,
or explicit rerun action. It snapshots mention expansion and authorization.

```yaml
id: DSP-18
source_type: comment
source_id: C-108
source_revision: 1
author_id: member-alice
direct_mentions:
  - security-team
expanded_members:
  - member-security
  - member-architect
  - member-bob
run_ids:
  - RUN-41
  - RUN-42
notification_ids:
  - N-9
created_at: timestamp
```

Dispatch status is derived from expansion and child results rather than used as
a separate workflow engine. Useful presentation states are:

```text
pending | active | completed | partial | rejected
```

## Run

A Run is one AI Member producing one response with one explicit Model.

```yaml
id: RUN-41
dispatch_id: DSP-18
subject_type: issue
subject_id: TOP-1#12
ai_member_id: member-security
conversation_id: CONV-7
identity_prompt_version: 3
model_id: claude-sonnet
context_snapshot_id: CTX-22
status: completed
conclusion: success
retry_of_run_id: null
```

A Run may contain multiple provider turns and read-tool calls:

```text
model turn
  -> search_text
  -> model turn
  -> read_file
  -> model turn
  -> final Comment
```

Those turns are internal steps of the same response, not separate Runs.

## Status and conclusion

Follow the GitHub Actions distinction:

```text
status: queued | in_progress | completed

conclusion:
  success | failure | cancelled | timed_out | skipped | neutral
```

`conclusion` is absent until status becomes `completed`.

- `success`: a final response was produced and persisted;
- `failure`: execution ended because of a non-recoverable error;
- `cancelled`: a user or policy stopped execution;
- `timed_out`: the Run exceeded its deadline;
- `skipped`: admission policy prevented execution;
- `neutral`: execution completed without a substantive response.

Success does not mean that the response is correct, approved, or adopted.

## Provider Attempts

An Attempt is one provider request or retry inside a Run. Automatic retry is
limited to failures that do not change the Model or semantic request, such as a
transient transport failure or explicitly classified rate limit.

```yaml
id: ATT-3
run_id: RUN-41
sequence: 2
provider_id: provider-anthropic
model_id: claude-sonnet
provider_request_id: optional
outcome: success
usage: {}
```

Changing Provider or Model always creates a new Run. It is not an Attempt.

## User retries

After a terminal failure, timeout, or unsatisfactory result, a user may retry.
That action creates a new Dispatch or a new Run, depending on scope:

- retry one AI Member -> new Run with `retry_of_run_id`;
- retry the original Team mention -> new Dispatch linked to the earlier Dispatch;
- rerun after edited context -> new Dispatch and new context snapshots.

Completed history remains unchanged.

## Output

A successful Run normally publishes one final Comment or Review attributed to
the AI Member. Streaming deltas, tool calls, and provider events appear in the
Run detail view rather than flooding the Issue timeline.

If the Run fails before producing a final response, its failure is visible as a
timeline event but it does not publish a fake AI Comment.

## Cancellation and concurrency

- one Conversation has at most one in-progress Run;
- different AI Members may run concurrently;
- cancelling a Dispatch requests cancellation of all unfinished child Runs;
- cancelling one Run does not cancel its siblings;
- late provider output after cancellation is retained as diagnostic data but is
  not published as a normal Comment;
- queued human input enters a Conversation only at the next safe turn boundary.

## First-version scope

- one Dispatch per source event;
- one Run per expanded AI Member;
- one Model per Run;
- multiple provider turns and read tools inside a Run;
- bounded same-model Attempts;
- user retry as a new Run or Dispatch;
- aggregate Dispatch status derived from child Runs and notifications;
- final output on the Issue timeline, detailed execution in the Run view.

## Current implementation

Dispatch resolution is implemented as one SQLite transaction. It snapshots each
resolved target and its direct or Team mention provenance, creates notifications
for Humans, reuses the `(TopicItem, AI Member)` Conversation, and persists a
queued Run plus a `run.execute` job for each available AI Member. Unknown handles,
empty Teams, inactive principals, and unavailable AI configuration are retained
as skipped targets; valid siblings still dispatch.

The provider-neutral execution service now claims `run.execute` jobs with a
five-minute lease, reclaims expired leases as a new Attempt, freezes context,
and atomically settles the Attempt, Run, Job, Conversation, activity event, and
final AI Comment. Provider failures produce a terminal failed Run and diagnostic
Conversation item without publishing a fake Comment.

Native HTTP adapters are not yet connected to the production worker, so normal
runtime Runs remain queued. The execution path is currently exercised through
an injected gateway in tests.
