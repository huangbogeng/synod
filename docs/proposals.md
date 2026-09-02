# Proposals

## Principle

A Proposal is a pull request for Topic knowledge. Humans and AI systems may
prepare one, but only a Human Member may merge it.

```text
Human / AI / caller ──> draft or open Proposal ──> review ──> human merge
```

Human-only merge is enforced by server authorization. It is not delegated to an
identity Prompt, model judgment, or client-side convention.

## States

```text
draft -> open -> merged
          |
          +-------> closed
draft ------------> closed
```

- `draft`: proposal is being prepared and cannot merge;
- `open`: proposal is ready for formal Review;
- `merged`: proposed changes were atomically added to the Topic main line;
- `closed`: proposal ended without changing the main line.

## Authorship

### Human Member

A Human Member with Topic write permission may create a Draft or open Proposal,
edit it, request Reviews, close it, reopen it, and merge it.

### AI Member

An AI Member may create a Draft or open Proposal when explicitly requested by a
human, caller, or authorized Run policy. It may update the Proposal, mark its
Draft ready for review, and respond to Review feedback but cannot:

- dismiss a blocking Human Review;
- merge it;
- impersonate a Human Member.

### External caller

Codex, CI, and other integrations use caller credentials. They may create,
update, and open Proposals within their scopes but cannot merge them.

## Reviews

Human and AI Members may submit:

```text
comment | approve | request_changes
```

AI Review state is advisory and visibly marked as such. Human Review state is
authoritative. An AI approval never satisfies a required Human approval.

Reviews bind to an exact Proposal revision. A content change makes earlier
approvals stale, while an unresolved Human `request_changes` remains blocking
until explicitly cleared. The complete Comment and Review contract is defined
in `docs/comments-and-reviews.md`.

The first version stores Review history but does not need a general policy
language. A Topic may use simple settings such as:

```yaml
required_human_approvals: 0
block_on_human_changes_requested: true
```

Whether these settings are configurable in the first release remains a product
decision. Human-only merge is unconditional regardless of their values.

## Merge authorization

The merge endpoint checks:

- authenticated principal type is `human`;
- the Human Member is active;
- the Human Member has Topic write permission;
- Proposal is `open`;
- base revisions are current;
- required Human Reviews, if configured, are satisfied;
- no unresolved blocking Human Review remains.

Caller tokens and AI Member credentials are rejected even if they can write
Comments, create Issues, or update Draft Proposals.

The audit event records the Human Member who merged. It never attributes a merge
to the AI Member that drafted the content.

## Merge effects

One successful merge transaction:

- creates a TopicRevision;
- applies all Document changes;
- accepts referenced Artifacts when requested;
- records linked decisions;
- closes Issues listed through `closes #...`;
- writes a merge event to the Topic timeline.

If any required write fails, none of these effects are committed.

## Direct edits are separate

Human-only merge does not prohibit direct human edits to Topic Documents. A
direct edit creates a TopicRevision without a Proposal. It is attributed to the
human editor and does not create a synthetic merge event.

Whether external callers may directly edit Topic Documents is a separate scope
decision. They still cannot merge Proposals.

## First-version defaults

- Human Member, AI Member, and external caller can create Draft or open Proposal
  within their permissions;
- only Human Member with write permission can merge;
- AI Reviews are advisory;
- unresolved Human `request_changes` blocks merge;
- merge applies changes atomically;
- merge actor and all resulting effects are auditable.
