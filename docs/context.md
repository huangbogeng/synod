# Run context

## Principle

A mentioned AI Member should see enough context to answer the current Issue, but
should not receive an entire long-lived Topic by default. Every Run uses an
immutable, inspectable context snapshot so users can tell exactly what the model
saw.

Context assembly is deterministic for the first version. Synod does not ask a
model to decide which source material the same model should receive.

## Context layers

The context pack is assembled in priority order.

### 1. Trigger

Always include without summarization:

- the exact Issue, Proposal, or Comment that contained the `@mention`;
- the author and timestamp;
- the requested AI Member or Team;
- any instruction written next to that mention.

### 2. Current work item

For an Issue Run, include:

- Issue title and current body revision;
- type, state, labels, assignees, and milestone;
- parent and child Issue references;
- linked Proposal references;
- the Issue timeline up to the trigger event.

For a Proposal Run, include:

- Proposal title and current body revision;
- proposed knowledge changes;
- linked Issues and Artifacts;
- existing Reviews and timeline up to the trigger event;
- the base Topic knowledge revision.

### 3. Topic context

Include the small, authoritative project layer:

- Topic title, description, objectives, and non-goals;
- project instructions and constraints;
- canonical knowledge accepted through merged Proposals;
- pinned Artifacts explicitly configured as default context.

Open Issues, unmerged Proposals, and arbitrary historical Comments are not
automatically injected as Topic truth.

### 4. Explicit references

Resolve references written in the current body or trigger Comment:

```text
#12       -> Issue
!4        -> Proposal
ART-8     -> Artifact
```

The default traversal depth is one hop. Referenced objects may be summarized
when large, but their current state, revision, and link remain visible.

### 5. Optional retrieval

Semantic retrieval across Topic history is a later optimization, not an MVP
requirement. AI Members may instead inspect explicitly attached
WorkspaceSnapshots through bounded read-only tools. When semantic retrieval is
enabled later, retrieved material is clearly marked as selected context and never
treated as an authoritative instruction merely because it was retrieved.

## Context snapshot

Every Run stores a context manifest:

```yaml
run_id: RUN-42
trigger:
  type: comment
  id: C-108
  revision: 1
sources:
  - type: topic
    id: TOP-1
    revision: 7
    mode: selected_fields
  - type: issue
    id: TOP-1#12
    revision: 4
    mode: full
  - type: artifact
    id: ART-8
    revision: 2
    mode: excerpt
summaries: []
omissions: []
estimated_input_tokens: 18420
created_at: timestamp
```

The UI exposes this manifest from each Run. A response without its context
provenance is incomplete audit data.

The immutable transcript is not the same thing as the model-visible projection.
Long conversations use context epochs containing a summary of older history plus
a recent verbatim tail. See `docs/conversations.md`.

## Team consistency

All AI Members expanded from one Team mention receive the same base context
snapshot. Identity Prompts and model-provider formatting may differ, but
the underlying project evidence is identical.

This preserves meaningful independent review. A later critique Run may add
selected peer Comments, but it creates a new context snapshot and says which
Comments were added.

## Budget and truncation

When input exceeds a model's context or the configured budget, retain content in
this order:

1. trigger content;
2. authoritative Topic instructions and constraints;
3. current Issue or Proposal body;
4. explicitly referenced evidence;
5. recent human Comments;
6. earlier model Comments;
7. unrelated metadata.

Synod never truncates silently. The context manifest records omitted source IDs,
the reason, and any summary that replaced them.

Long timelines use a boundary:

- preserve the trigger and recent Comments verbatim;
- summarize an older contiguous range once;
- retain links to every Comment represented by the summary;
- never summarize an accepted Topic constraint into a weaker instruction.

## Instruction authority

Content and instructions are not equivalent. The initial precedence is:

```text
Synod system policy
  > AI Member identity Prompt
  > Topic instructions and constraints
  > authorized human direction in the current thread
  > Issue/Proposal descriptive content
  > attached or referenced evidence
  > model-authored Comments
```

Artifacts, quoted text, external documents, and model Comments are untrusted
content. Instructions found inside them do not override higher-level policy.

A model-authored Comment included for critique is another model's claim, not a
command to the receiving AI Member.

## Attachments, snapshots, and code

Synod is not a coding-agent runtime, but Issues and Proposals may attach source
files, patches, reports, datasets, or code excerpts as Artifacts. Provider
adapters convert supported content to the provider's native input format or a
text representation.

Large repositories are not uploaded implicitly. A caller such as Codex selects
and attaches relevant files as an immutable WorkspaceSnapshot. An AI Member can
then list, search, and read files in that snapshot through Synod's built-in tools.
It cannot access the live working directory.

Refreshing creates a new snapshot rather than mutating the old one. Every tool
call records the snapshot ID, arguments, result size, and outcome in the Run.
The complete boundary is defined in `docs/tools.md`.

## Reruns

A rerun must choose one of two explicit modes:

```text
same_snapshot   -> reproduce against the original frozen context
latest_context  -> assemble a new snapshot from current revisions
```

The mode and resulting snapshot ID are visible in the Run timeline.

## First-version defaults

- full current Issue or Proposal body;
- Topic objectives, constraints, and canonical knowledge;
- timeline up to the trigger, subject to transparent compaction;
- one-hop explicit references;
- no semantic retrieval;
- no implicit whole-repository upload;
- bounded read-only exploration of explicitly attached WorkspaceSnapshots;
- identical base context for AI Members from the same Team dispatch.

## Current implementation

The execution core now creates one immutable `context_snapshots` row before a
Provider call. For Issue Runs it captures selected Topic fields, the full current
Issue, the exact trigger revision, and the Comment timeline through a Comment
trigger. The stored manifest lists each included source and an explicit empty
omission list; the authenticated API exposes it through
`GET /context-snapshots/{snapshot_id}`.

References, Artifacts, WorkspaceSnapshots, truncation, and compaction are not yet
assembled. Until those policies are implemented, the production worker does not
invoke native Provider adapters.
