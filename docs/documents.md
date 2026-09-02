# Topic Documents and revisions

## Principle

Topic Documents form a continuously improving main line of project knowledge.
They may be edited directly or changed through a reviewed Proposal.

```text
direct edit ---------+
                     +--> TopicRevision --> current Documents
Proposal merge ------+
```

Proposal is optional. It is useful when a change benefits from explicit review,
not a mandatory ceremony for every correction or incremental improvement.

Regardless of authorship or review outcome, only a Human Member may merge a
Proposal into the main line.

## Documents

A Topic contains a small virtual document tree:

```text
README.md
research-plan.md
decisions/
findings/
protocols/
```

Documents are Synod records, not files in a Git repository. The first version
supports Markdown content and these operations:

```text
create | update | rename | archive | restore
```

Archive is used instead of destructive deletion. Historical revisions keep the
old path and content available.

## TopicRevision

Every saved change creates an immutable TopicRevision:

```yaml
id: REV-18
topic_id: TOP-1
parent_revision_id: REV-17
author_id: member-alice
source_type: direct_edit
source_id: optional
message: Clarify missing-timestamp policy
changes:
  - document_id: DOC-4
    operation: update
    old_revision: 6
    new_revision: 7
created_at: timestamp
```

The Topic has one linear main history. Synod does not implement general-purpose
branches or a commit graph.

## Direct edits

An authorized Human Member or external caller may edit Documents directly.

```text
open editor
  -> edit a draft locally
  -> Save changes
  -> create one TopicRevision
```

Autosave may preserve an editor draft, but it must not create a TopicRevision for
every keystroke. Saving requires a short change message, which may be generated
and edited by the user.

Direct edits use optimistic concurrency:

```yaml
document_id: DOC-4
base_revision: 6
new_content: ...
```

If the current revision is no longer 6, the save is rejected as stale and the
editor offers comparison and reapplication. The server never silently overwrites
newer content.

AI Members do not directly edit the main line by default. They may suggest a
patch in a Comment or create a draft Proposal when explicitly asked. A later
permission policy may allow selected AI Members to direct-edit, but that is not
part of the first version.

## Proposal changes

A Proposal captures full proposed document revisions against a base:

```yaml
base_topic_revision: REV-17
changes:
  - document_id: DOC-4
    base_document_revision: 6
    proposed_content: ...
```

The UI computes a diff for review. Synod stores base and proposed content rather
than treating a textual patch as the canonical change.

When main advances, only Proposals touching changed Documents become outdated.
Unrelated direct edits do not block merge.

The first version does not perform automatic three-way merging. The Proposal
author refreshes the proposed Document or explicitly reapplies it to the latest
revision.

## Merge

Merging a Proposal applies all of its changes atomically and creates one
TopicRevision:

```yaml
source_type: proposal_merge
source_id: PROP-4
```

If any touched Document has a stale base revision, none of the changes are
applied. Linked `closes #...` actions occur only after the document transaction
succeeds.

## History and restore

Users can inspect:

- Topic revision history;
- Document revision history;
- author and change message;
- direct edit or Proposal source;
- before/after diff;
- Issues and Artifacts referenced by the change.

Restore is implemented as a new TopicRevision that writes an earlier Document
version back onto main. Existing history is never removed.

## Run context

Every Run records the exact TopicRevision and Document revisions included in its
context snapshot. Later direct edits affect new Runs only; they do not change what
a completed or in-progress Run saw.

## First-version scope

- Markdown Documents;
- virtual paths;
- one linear main history;
- immutable TopicRevisions;
- direct edits by humans and authorized callers;
- draft editing separated from saved revisions;
- optimistic concurrency;
- Proposal diff and atomic merge;
- restore through a new revision;
- no branches, rebases, or automatic merge algorithm.
