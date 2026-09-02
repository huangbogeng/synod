# GitHub-shaped workflow

## Principle

Synod uses the GitHub collaboration model directly. A Topic behaves like a
repository, an Issue captures work to understand or resolve, and a Proposal
submits a concrete change for review. Model activity appears in the same
timeline as human activity but remains attributable to a Run and explicit Model.

## Object graph

```text
Topic
├── Issues
│   ├── child Issues
│   ├── Comments
│   ├── Runs
│   └── linked Proposals
├── Proposals
│   ├── Comments
│   ├── Reviews
│   ├── Runs
│   └── proposed knowledge changes
└── Artifacts
```

## 1. Topic: the project

A Topic is a long-lived project such as developing a model or researching a
class of quantitative factors. It owns:

- title, description, objectives, and non-goals;
- project-wide constraints and instructions;
- labels and milestones;
- canonical knowledge accepted through merged Proposals;
- directly editable Documents on a linear main revision history;
- Issues, Proposals, Artifacts, and the activity timeline.

Topic state is intentionally small:

```text
active | archived
```

Project success, failure, or abandonment is recorded as an explicit outcome,
not overloaded into infrastructure state.

## 2. Issue: the unit of work

An Issue may represent a question, task, hypothesis, experiment, risk, or bug.
It contains a title, description, author, assignees, labels, milestone, Comments,
linked Artifacts, mentions, and relationships.

Issue state follows GitHub:

```text
open | closed
```

A closed Issue records a reason:

```text
completed | not_planned | duplicate
```

Issues may split into child Issues. Each child has at most one parent so the
primary hierarchy stays understandable. Additional links express:

```text
blocks | blocked_by | related_to | duplicates
```

Closing a parent does not silently close its children. A Proposal or explicit
human action must state which Issues it resolves.

### Issue types

An Issue has one primary type. Synod ships a small default set:

| Type | Typical purpose |
| --- | --- |
| `task` | A concrete piece of work not covered by a more specific type |
| `bug` | Unexpected or incorrect behavior |
| `feature` | A proposed capability or user-visible improvement |
| `code_audit` | Structured review of code quality, correctness, security, or performance |
| `research` | An open question or hypothesis requiring evidence |
| `experiment` | A bounded protocol and its resulting evidence |
| `decision` | A choice among explicit alternatives |

Topic owners may define additional types. An Issue type can provide:

- a creation template or structured fields;
- default labels and suggested AI Members or Teams;
- suggested Run workflow;
- completion guidance;
- a Proposal template;
- optional automation rules.

Types do not change the universal `open | closed` lifecycle. They provide
defaults, not hidden behavior. A user can override the suggested mentions or Run.

Type and labels serve different purposes:

```text
type: code_audit
labels: [security, authentication, high-risk]
```

The type is the Issue's primary workflow intent. Labels support filtering and
cross-cutting classification.

Child Issues may use a different type from their parent. This is essential for
natural decomposition:

```text
#20 [code_audit] Audit authentication module
├── #21 [bug] Refresh token can be replayed
├── #22 [feature] Add session revocation
└── #23 [task] Add concurrency regression tests

#30 [research] Investigate analyst revision signals
├── #31 [experiment] Test publication-time coverage
├── #32 [decision] Select the missing-timestamp policy
└── #33 [code_audit] Audit PIT implementation
```

## 3. Comments: the shared conversation

Humans, AI Members, and external callers use the same Comment object. Every
Comment records its author type and provenance:

```yaml
author_type: human | ai | caller | system
run_id: optional
model_route: optional
reply_to: optional
created_at: timestamp
edited_at: optional
```

Comments may carry a presentation kind without becoming separate top-level
objects:

```text
discussion | direction | evidence | progress | result
```

A human direction is therefore a durable Comment. It does not rewrite earlier
Comments. If it changes the premise materially, a new Run uses a new immutable
input snapshot.

### Mentions trigger work

An `@mention` in an Issue body, Proposal body, or Comment is the primary dispatch
mechanism:

```text
@architect               -> start one model Run
@security-review-team    -> fan out to the Team's AI members
@alice                   -> notify a human member
```

Issue types may suggest AI Members in the editor, but they do not silently start
model work. The author chooses who participates by mentioning them.

Team dispatch snapshots membership and AI Member configuration at trigger time.
Later Team changes do not alter an existing Run. AI Member invocations may run
concurrently, but Team does not define a discussion mode or ordering policy.

Mention behavior, deduplication, permissions, and loop prevention are specified
in `docs/mentions.md`.

## 4. Runs: observable model work

A Run is one bounded response by one AI Member using one explicit Model, attached
to an Issue or Proposal. A Team mention first creates one Dispatch and then one
Run for every expanded AI Member. Examples:

- ask several AI Members for independent opinions;
- analyze disagreements in existing Comments;
- review a Proposal for security and performance;
- synthesize a proposed resolution;
- rerun after new evidence or human direction.

Run status and conclusion follow GitHub Actions-style separation:

```text
status: queued | in_progress | completed
conclusion: success | failure | cancelled | timed_out | skipped | neutral
```

Runs record frozen inputs, one AI Member, one resolved Model, provider Attempts,
token usage, errors, generated Comments, and Artifacts. The Run may contain
several model turns and read-tool calls while producing that response.

A successful Run means only that the workflow finished. It does not close an
Issue or approve a Proposal.

Transient transport retries remain Attempts inside one Run. Retrying a terminal
Run by human action creates a new Run linked through `retry_of_run_id`.

## 5. Splitting Issues

When a Run discovers several independent questions, it may suggest child Issues.
Creating them requires explicit human action, prior authorization on the Run,
or an automation rule configured on the Topic.

The parent records why each child was created. Models cannot recursively create
unbounded work: Topic policy limits depth, child count, and total Run budget.

Example:

```text
#12 Can consensus-estimate data satisfy PIT requirements?
├── #15 What is the vendor's actual publication timestamp?
├── #16 Does historical data contain later backfills?
└── #17 What is the policy for records without timestamps?
```

## 6. Proposal: the pull request for decisions

An Issue describes what needs to be resolved. A Proposal describes a concrete
resolution and the changes it would make to canonical Topic knowledge.

A Proposal contains:

- summary and motivation;
- proposed conclusions or policy changes;
- evidence and linked Artifacts;
- assumptions and risks;
- alternatives rejected and reasons;
- local checks still required;
- linked Issues, including `closes #123` semantics;
- the base Topic knowledge revision it was prepared against.

Proposal state follows pull requests:

```text
draft | open | merged | closed
```

If Topic knowledge changed after the Proposal's base revision, the UI marks it
outdated and requires refresh or explicit confirmation before merge.

Proposal is an optional review path, not the only way to update Topic Documents.
Authorized humans and callers may edit the main line directly. Both a direct
edit and a Proposal merge create an immutable TopicRevision.

## 7. Reviews

Humans and configured AI Members may review a Proposal:

```text
comment | approve | request_changes
```

Topic policy determines which approvals are required. Model approval is advice;
human approval remains distinct and cannot be silently substituted by another
model.

Each Review targets an exact Proposal revision. A later content change makes
prior approvals stale. An unresolved Human `request_changes` continues to block
merge until the reviewer changes the verdict or an authorized Human Member
dismisses it with an audit reason. AI Reviews never create or remove a merge
block.

Dismissal is a separate audit event and does not rewrite the original Review.

AI Members and external callers may create, update, and open Proposals within
their granted permissions. Only an authenticated Human Member with Topic write
permission may merge a Proposal. Caller and AI credentials are rejected by the
merge command even when they otherwise have permission to create content.

Review Comments should point to a specific proposed claim, knowledge change, or
evidence item where possible, mirroring line-level review without pretending the
Proposal is source code.

Ordinary Comments carry no approval or merge semantics. The first version uses
a flat timeline with an optional single reply reference and omits line-level
review threads. See `docs/comments-and-reviews.md`.

## 8. Merge

Merging a Proposal is the formal write boundary. It may:

- add or revise canonical Topic knowledge;
- record an accepted decision;
- attach accepted Artifacts;
- close explicitly linked Issues;
- create follow-up Issues listed in the Proposal.

For a Proposal, merge is its formal write boundary. Topic Documents also permit
explicit direct edits, which produce the same immutable revision record without
a Proposal or Reviews.

Merge never means that code was implemented, an experiment was run, or a result
was published unless corresponding evidence is attached and explicitly
accepted. External callers such as Codex remain responsible for local execution
and may post resulting evidence back to the Topic.

## 9. Suggested CLI shape

```bash
# Topic
synod topic create --title "Analyst revision factor research"
synod topic view TOP-1

# Issue
synod issue create --topic TOP-1 --title "Verify PIT feasibility" --body issue.md
synod issue create --parent TOP-1#12 --title "Check vendor timestamps"
synod issue comment TOP-1#12 --body comment.md

# Run
synod run start TOP-1#12 --panel data-review
synod run view RUN-42
synod run cancel RUN-42

# Proposal and review
synod proposal create --topic TOP-1 --body proposal.md --closes 12
synod proposal review TOP-1!4 --approve
synod proposal review TOP-1!4 --request-changes --body review.md
synod proposal merge TOP-1!4
```

Exact identifiers are not frozen, but GitHub-like `#issue` and `!proposal`
references keep conversation compact and linkable.

## 10. MVP

The first useful version needs only:

1. Topics;
2. typed, nested Issues and Comments;
3. mention-triggered, provider-neutral model Runs;
4. Proposals and Reviews;
5. a mergeable canonical-knowledge change set;
6. one timeline shared by Web, CLI, and HTTP;
7. Markdown and JSON rendering for external orchestrators.

Milestones, notifications, saved searches, project boards, reactions, and rich
permissions can wait until the core Issue-to-Proposal loop is proven useful.
