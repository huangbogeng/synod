# Core data model

## Design rules

The first version uses a relational database as the source of truth. The schema
favors explicit ownership, immutable history, and ordinary transactions over a
general event-sourcing framework.

- internal primary keys are opaque UUIDs;
- user-facing Topic keys and item numbers are stable locators;
- mutable objects carry an integer `revision` for optimistic concurrency;
- accepted knowledge, model inputs, Reviews, Runs, and audit records are never
  rewritten in place;
- actor identity always comes from authenticated server context;
- timestamps are stored in UTC.

## Relationship overview

```text
Principal ──< TopicMembership >── Topic ──< TopicItem
   │                                  │          ├── Issue
   │                                  │          └── Proposal ──< ProposalRevision
   │                                  │                              ├── ProposalChange
   │                                  │                              └── Review
   │                                  │
   │                                  ├──< Document ──< DocumentVersion
   │                                  ├──< TopicRevision ──< TopicRevisionChange
   │                                  ├──< Artifact
   │                                  └──< Team ──< TeamMember
   │
   └── authors Comments and authenticated actions

TopicItem ──< Comment ──< CommentRevision
TopicItem ──< Conversation ──< Run ──< Attempt
TopicItem ──< Dispatch ──< DispatchTarget ──> Run / Notification
Topic     ──< ActivityEvent
```

## Principals and membership

`principals` is the common actor table:

```text
Principal
  id
  kind: human | ai | caller | system
  handle
  display_name
  active
  created_at
```

Subtype tables hold kind-specific data:

- `human_profiles`: authentication-account linkage;
- `ai_profiles`: current Prompt version and default Model;
- `caller_profiles`: integration metadata; secrets are stored outside normal
  application rows;
- `ai_prompt_versions`: immutable Prompt text and version number.

`topic_memberships(topic_id, principal_id, role)` grants `read`, `contribute`,
or `write`. A role describes ordinary content permissions. Merge additionally
requires `principal.kind = human`; a write-capable AI or caller still cannot
merge.

Teams are Topic-scoped static groups. `team_members` accepts Human and AI
principals only. Team membership never grants permission beyond the member's
own Topic membership.

## Topics and items

`topics` contains the project-level identity, settings, and current main
revision. Each Topic owns a monotonically allocated public item number.

`topic_items` is an internal relational base for objects with a discussion
timeline:

```text
TopicItem
  id
  topic_id
  number
  kind: issue | proposal
  title
  author_principal_id
  revision
  created_at
  updated_at
```

`UNIQUE(topic_id, number, kind)` supports public locators such as `TOP-1#12` and
`TOP-1!4`. Issue and Proposal remain distinct API resources and subtype tables;
`TopicItem` is not a user-visible product concept.

`issues` adds:

```text
item_id
type_id
state: open | closed
body
parent_issue_item_id: nullable
closed_by_principal_id: nullable
closed_at: nullable
```

Labels, assignees, relations, milestones, and closing references use join tables
instead of arrays embedded in an Issue row.

`proposals` adds:

```text
item_id
state: draft | open | merged | closed
current_proposal_revision
base_topic_revision_id
merged_by_principal_id: nullable
merged_topic_revision_id: nullable
merged_at: nullable
```

The database constrains `merged_by_principal_id` to a Human principal through
the merge transaction. API and domain authorization perform the same check
before entering that transaction.

## Comments and Reviews

`comments` references one `topic_item`. Its current body and revision support
editing, while `comment_revisions` preserves every prior body. A deletion sets a
tombstone state and never removes the row.

`reviews` is append-only and references:

- one Proposal item;
- one exact Proposal revision;
- one Human or AI reviewer principal;
- one verdict: `comment`, `approve`, or `request_changes`;
- an optional source Run.

`review_dismissals` is a separate append-only table containing the Review,
Human dismissing principal, reason, and timestamp. The original Review is not
mutated.

## Documents and accepted knowledge

`documents` owns stable identity, current path, current version number, and
archive state. `document_versions` stores immutable Markdown content.

`topic_revisions` is the linear main history. Each row references its parent,
actor, source, and message. `topic_revision_changes` maps the revision to old and
new Document versions.

`proposal_revisions` stores the immutable Proposal body and metadata for each
saved revision. `proposal_changes` stores each proposed full Document version
and its base Document version. A diff is derived, not canonical.

Proposal merge locks the Proposal and touched Documents, verifies bases and
Human authorization, writes Document versions and one Topic revision, records
the merge actor, and applies closing references in one transaction.

## Model configuration and execution

`providers` and `models` store non-secret configuration. A Model belongs to one
Provider. Credentials are referenced by opaque identifiers; optional local
secret values live separately in `provider_secrets` and never cross a read API.

One `conversation` is unique for `(topic_item_id, ai_principal_id)`. It owns the
provider-neutral transcript and current context epoch. Transcript entries are
append-only.

`dispatches` snapshot a mention source and source revision. `dispatch_targets`
snapshot expansion into individual Human notifications or AI Runs. Later Team
membership changes do not rewrite them.

One `run` belongs to one AI target, Conversation, Model, Prompt version, and
context snapshot. `attempts` record provider requests and same-model retries.
Tool calls and normalized stream events belong to a Run and do not become
top-level timeline Comments.

## Artifacts and workspace snapshots

`artifacts` stores metadata and an immutable content digest; large bytes may
live in object storage. A content-addressed blob may be referenced by several
Artifacts without merging their provenance.

A `workspace_snapshot` is an immutable manifest. `workspace_snapshot_files`
maps normalized virtual paths to immutable blobs. Snapshot creation rejects
absolute paths, parent traversal, duplicate normalized paths, and content whose
digest does not match the manifest.

## Timeline and audit

`activity_events` is an append-only presentation and audit index:

```text
id
topic_id
sequence
item_id: nullable
event_type
actor_principal_id
subject_type
subject_id
payload
created_at
```

`UNIQUE(topic_id, sequence)` provides deterministic cursor pagination. Events
reference domain records; they are not the sole storage of domain state.
Sensitive provider payloads and secrets are excluded from event payloads.

## Required invariants

- every Issue, Proposal, Comment, Review, Run, and referenced Artifact belongs
  to the same Topic unless an explicit cross-Topic reference is authorized;
- a child Issue cannot be its own ancestor;
- one Conversation has at most one in-progress Run; additional accepted Runs
  may wait in its durable queue;
- one source revision produces at most one Dispatch unless the action is an
  explicit rerun;
- one Dispatch produces at most one target per expanded principal;
- an Attempt cannot change its parent Run's Provider or Model;
- completed Runs, Reviews, revisions, Attempts, and activity events are
  immutable;
- only a Human writer can merge;
- Topic main history is linear;
- Proposal merge is all-or-nothing.

These invariants belong in domain services and database constraints where the
database can express them reliably.
