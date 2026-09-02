# Comments and Reviews

## Principle

Synod separates discussion from formal judgment:

```text
Comment = discussion
Review  = formal judgment on one Proposal revision
```

Both appear in the same chronological timeline. A Review has additional state
and merge semantics; a Comment never does.

## Comments

A Comment belongs to exactly one Issue or Proposal and contains:

```yaml
id: COM-42
subject: TOP-1#12 | TOP-1!4
author: member-or-caller-id
author_type: human | ai | caller | system
body: markdown
kind: discussion | direction | evidence | progress | result
reply_to_comment_id: optional
source_run_id: optional
created_at: timestamp
edited_at: optional timestamp
```

Comments are used for questions, evidence, directions, model answers, and
ordinary discussion. They cannot approve a Proposal, request changes, close an
Issue, or alter Topic Documents.

`kind` is presentation metadata for filtering and rendering. It does not create
different Comment classes or grant additional authority.

Comments may be edited, but every saved version remains available in the audit
history. Deletion is represented by a tombstone so replies, Runs, and audit
events do not lose their referent.

The first version supports an optional `reply_to_comment_id` for context, while
rendering the main conversation as a flat chronological timeline. It does not
implement arbitrary nested threads.

## Reviews

A Review belongs to one Proposal and one exact Proposal revision:

```yaml
id: REVW-7
proposal_id: TOP-1!4
proposal_revision: 3
reviewer: member-id
reviewer_type: human | ai
verdict: comment | approve | request_changes
body: markdown
source_run_id: optional
created_at: timestamp
```

- `comment` records a formal review without a merge decision;
- `approve` accepts the reviewed Proposal revision;
- `request_changes` identifies changes required before merge.

External callers may add Comments, but cannot submit Reviews under their own
identity in the first version. A caller may explicitly invoke an AI Member, and
the resulting Run may submit an attributable AI Review.

## Human and AI authority

Human Reviews are authoritative. AI Reviews are advisory and are always marked
with the AI Member and Run that produced them.

An AI Review:

- never satisfies a required Human approval;
- never creates a blocking merge condition;
- cannot dismiss or replace a Human Review;
- remains useful as visible analysis and provenance.

A Human `request_changes` remains blocking until either:

- the same reviewer submits a later non-blocking verdict; or
- another Human Member with Topic write permission dismisses it with a reason.

Dismissal is an audit event, not a rewrite or deletion of the original Review.

## Proposal revisions

Every Proposal content change increments `proposal_revision`. Reviews retain the
revision they examined.

When a Proposal changes:

- existing approvals become stale and do not satisfy approval requirements;
- existing AI Reviews remain visible as historical advice;
- unresolved Human `request_changes` remains blocking until explicitly cleared;
- reviewers may submit a new Review against the new revision.

This prevents an author from bypassing a change request through a trivial edit.

## Mentions and Runs

Mentions inside a new or edited Comment follow the normal Dispatch rules. A
model-generated Comment or Review must reference its `source_run_id`.

Model-authored mentions remain inert suggestions by default, including mentions
inside AI Reviews. This prevents recursive review loops.

## First-version UI

Issue timeline:

```text
body -> Comments -> Runs -> state events
```

Proposal timeline:

```text
body/revisions -> Comments -> Runs -> Reviews -> merge/state events
```

The UI may filter AI or Human contributions, but storage and APIs use one shared
timeline ordered by server-assigned event sequence.

The first version deliberately omits:

- line-level and range-level review comments;
- arbitrary nested discussion threads;
- reactions as decision signals;
- AI votes or quorum calculations;
- automatic resolution of review findings.
