# Product boundary

## The problem

A coding orchestrator usually has strong local context but can benefit from
independent architectural, security, performance, and product criticism. Putting
all discussion into its main context is expensive and noisy. Running several
autonomous coding agents is unnecessarily broad when the actual need is review,
research, and deliberation.

Synod keeps that work in a durable project space and exposes the result back to
the caller through familiar GitHub-shaped collaboration objects.

## Independent implementation

Synod is not a fork, distribution, or extension of Multica. Public products are
used only to identify useful interaction patterns and avoidable complexity.
Synod designs its own domain model, protocols, interfaces, and implementation.

- no competitor source code is copied or vendored;
- no competitor package, server, daemon, or CLI is a runtime dependency;
- compatibility with those products is not a goal;
- all implementation and contracts originate in this repository.

Synod is intended to be fully open source and non-commercial. A specific
open-source license will be selected separately.

## What we retain from existing products

- one durable project space connects intent, discussion, runs, and results;
- every model invocation has an observable execution record;
- human and model contributions share a chronological timeline;
- run completion and project completion are distinct;
- Web, CLI, MCP, and HTTP expose the same server-side objects;
- self-hosting and explicit data boundaries are first-class concerns;
- familiar Issue and pull-request workflows reduce learning cost.

## What we remove

- coding-agent CLI adapters;
- connected machines and local daemons;
- repositories, worktrees, diffs, and code merges;
- general project-management suites;
- reusable coding skills and autonomous agent squads;
- cron-driven software delivery;
- tool execution and tool approval.

These belong to callers and integrations outside Synod.

## GitHub-shaped abstraction set

| Object | Responsibility |
| --- | --- |
| `Topic` | Project-level container, analogous to a GitHub repository |
| `Issue` | A question, task, hypothesis, or problem inside a Topic |
| `Proposal` | A reviewable resolution, analogous to a pull request |
| `Comment` | Human, model, or system contribution to an Issue or Proposal |
| `Review` | Formal approval, change request, or review comment on a Proposal |
| `Dispatch` | One durable expansion of mentions from a source event |
| `Run` | One observable model workflow, analogous to a GitHub Actions run |
| `Conversation` | Provider-neutral long conversation for one AI member on one Issue or Proposal |
| `Artifact` | Versioned input, evidence, report, or generated result |
| `IssueType` | Configurable Issue classification with templates and workflow defaults |
| `WorkspaceSnapshot` | Immutable file bundle exposed through built-in read-only tools |
| `Document` | Mutable canonical project knowledge with immutable revision history |
| `TopicRevision` | One atomic change on the Topic's linear main history |

Identity and model access use these additional objects:

| Object | Responsibility |
| --- | --- |
| `Member` | Mentionable human or AI participant |
| `Team` | Mentionable static group of Members; no routing or workflow behavior |
| `Provider` | API protocol, endpoint, and credentials reference |
| `Model` | Explicit model configuration and capability declaration |

There is intentionally no autonomous `Agent` or separate `Reviewer` object in
the core model. An AI Member is an identity Prompt plus a default Model and its
Conversations. It has no persistent runtime or implicit authority. Its only
optional tools are Synod-owned read operations over immutable Topic data and
WorkspaceSnapshots.

AI Members and Teams are directly mentionable from Issue and Proposal text.
Mentions are explicit dispatch commands rather than hints to a hidden router.

Only an authenticated Human Member with Topic write permission may merge a
Proposal. This is an application authorization rule, not an instruction that an
AI Member is expected to follow voluntarily.

## Mapping to GitHub

```text
GitHub repository    -> Topic
GitHub issue         -> Issue
GitHub sub-issue     -> child Issue
GitHub pull request  -> Proposal
Issue/PR comment     -> Comment
PR review            -> Review
GitHub Actions run   -> Run
attachment/artifact  -> Artifact
```

Synod follows GitHub's interaction model, not its implementation. Familiar
status names, timelines, references such as `#123`, labels, assignees, and
notifications are preferable to novel multi-agent terminology.

Issue types are first-class and extensible. An Issue has one primary type while
labels remain many-to-many descriptive metadata. Types may suggest templates,
AI Members, Teams, and workflows, but do not create separate Issue tables or
incompatible lifecycle rules.

## Integration boundary

```text
CLI -----+
MCP -----+--> Synod application API --> model workflow engine --> providers
HTTP ----+
```

CLI and MCP are clients, not separate orchestration implementations. State
transitions, permission checks, idempotency, and output contracts remain shared.

## Provider neutrality

The engine consumes a small internal interface:

```text
generate(request) -> response
stream(request)   -> events
capabilities()    -> model capabilities
```

The first Provider adapter translates normalized requests only for the official
DeepSeek and MiniMax OpenAI-compatible endpoints. Other vendors may be added
behind the same interface after their protocol is implemented. AI Members do not
bind to tools. Runs record the identity Prompt version, explicit Model,
parameters, and provider attempts. Automatic cross-model fallback is not part of
the first version.

## Non-goals for the first version

- autonomous code changes;
- arbitrary, user-supplied, or third-party tool calling by AI members;
- file writes, shell execution, network requests, or repository mutation;
- hidden cross-Issue AI-member memory;
- open-ended group chat;
- treating model consensus as proof;
- replacing human or caller-side verification;
- reproducing every GitHub project-management feature.
