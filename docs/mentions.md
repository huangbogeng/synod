# Mentions and dispatch

## Principle

Synod follows GitHub's direct collaboration model: mention the person or team
whose attention is needed. There is no required panel-selection screen and no
implicit leader that decides who should respond.

```text
@ai       -> invoke that model-backed member
@team     -> invoke AI members and notify human members
@human    -> notify that member
```

## Mentionable identities

### Human member

A workspace user. Mentioning a human creates a notification and never invokes a
model on that person's behalf.

### AI member

A model-backed member such as `architect`, `security-reviewer`, or
`quant-researcher`. Its identity is a versioned Prompt and it selects a default
Model. It does not own a machine or a persistent process.

### Team

A named group containing human and AI Members. A Team has no leader, Prompt,
router, workflow, or discussion mode. Mentioning it expands the current member
list: AI Members are invoked independently and Human Members are notified.

Teams cannot contain other Teams in the first version. A Member may belong to
several Teams.

## Trigger sources

Mentions are parsed from:

- a newly created Issue body;
- a newly created Proposal body;
- a new Comment;
- an explicit rerun command targeting an earlier mention event.

The server persists the source object before enqueueing Runs. A trigger therefore
always points to durable content that can be reconstructed and audited.

The current implementation persists an ordered, deduplicated mention snapshot
and pending Dispatch atomically with each new Issue or Comment. Member and Team
resolution, notifications, and Run creation are the next execution layer; a
pending Dispatch does not yet imply that a model was invoked.

## Trigger behavior

One source event creates one dispatch record. The dispatch record contains:

```yaml
source_type: issue | proposal | comment
source_id: string
author_type: human | ai | caller | system
mentions: []
team_snapshot: []
created_runs: []
status: pending | dispatched | partially_dispatched | rejected
```

One expanded AI Member creates one Run. Expanded Human Members create
notifications but no Runs. The complete boundary is defined in `docs/runs.md`.

Repeated appearances of the same handle in one source event invoke it once.
Overlapping mentions are also deduplicated: if `@architect` is mentioned directly
and is also a member of `@design-team`, only one Run is created for that AI Member.

Each model Run receives an immutable snapshot containing:

- the source content;
- relevant Topic instructions;
- the Issue or Proposal timeline allowed by context policy;
- referenced Artifacts;
- AI Member identity Prompt;
- resolved Provider and Model;
- the dispatch event and Team membership snapshot.

The complete assembly and truncation rules are defined in `docs/context.md`.

## Model-authored mentions

Unrestricted model-authored mentions can create infinite loops and unbounded
cost. The default rule is:

- human-authored mentions dispatch immediately;
- authenticated external-caller mentions dispatch when the caller has permission;
- model-authored mentions render as suggestions and do not dispatch;
- a Run may opt into bounded model handoff with an explicit policy and remaining
  depth, invocation, and token budgets.

The UI distinguishes an inert suggested mention from a dispatched mention. A
human can activate the suggestion with one action.

## Edits and reruns

Editing existing text never silently invokes models again. The edit is recorded,
and the author may explicitly rerun the previous dispatch against the new
revision. This prevents correcting a typo from unexpectedly spending money or
producing duplicate Comments.

## Permissions

Dispatch requires permission to invoke every targeted AI Member. Team membership
does not bypass Member access rules. A Team dispatch may be partial when some
members are unavailable or unauthorized; the dispatch record lists every skip
and reason.

Credential access is resolved server-side through Provider. Mention authors
never receive provider credentials.

## Failures

Failure of one Team member does not discard successful responses from others.
The Team dispatch becomes `partially_dispatched` or completes with a partial Run
conclusion, and the timeline exposes which AI Member failed, timed out, or was
skipped.

## Examples

Issue body:

```markdown
## Goal

Audit the authentication redesign before implementation.

@architect review module boundaries and migration risks.
@security-review-team focus on token replay and privilege escalation.
```

Follow-up Comment:

```markdown
The deployment must remain single-node.

@architect reassess the recommendation without Redis.
```

The second mention starts a new Run using the updated Comment as its trigger. It
does not rewrite or invalidate the first response.
