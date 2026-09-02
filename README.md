# Synod

Synod is a lightweight, model-provider-neutral collaboration hub for AI-assisted
research and software decisions.

Its interaction model deliberately follows GitHub. A project is a Topic, a
question or task is an Issue, a concrete resolution is a Proposal, and model
work is recorded as observable Runs, Comments, and Reviews.

## Product boundary

Synod coordinates **reasoning**, not coding-agent execution.

- Codex, Claude Code, a CI job, or a person may call Synod through CLI, MCP, or
  HTTP.
- AI members are ordinary members backed by an identity Prompt and a configured
  model-provider API.
- AI members may inspect immutable workspace snapshots through a small set of
  built-in read-only tools.
- Synod does not spawn coding CLIs, edit repositories, run shell commands,
  manage worktrees, or merge code.
- The caller remains responsible for local verification and implementation.

Synod is an independent open-source implementation. Other projects are studied
only for product patterns and lessons; their source code is not copied, vendored,
or used as an implementation dependency.

## GitHub-shaped model

```text
GitHub repository    -> Topic
GitHub issue         -> Issue
GitHub sub-issue     -> child Issue
GitHub pull request  -> Proposal
Issue/PR comment     -> Comment
PR review            -> Review
GitHub Actions run   -> Run
```

## Core flow

```text
select a Topic
      |
      v
open an Issue
      |
      v
@mention AI members or teams
      |
      v
model Runs add Comments and may suggest child Issues
      |
      v
open a Proposal resolving one or more Issues
      |
      v
human/model Reviews approve or request changes
      |
      v
merge accepted conclusions into Topic knowledge
      |
      v
caller verifies locally and executes
```

See [docs/product-boundary.md](docs/product-boundary.md) and
[docs/deliberation-workflow.md](docs/deliberation-workflow.md). The default and
extensible Issue taxonomy is defined in [docs/issue-types.md](docs/issue-types.md).
Mention-driven dispatch is defined in [docs/mentions.md](docs/mentions.md).
Run input assembly and auditability are defined in
[docs/context.md](docs/context.md).
The constrained read-only tool boundary is defined in [docs/tools.md](docs/tools.md).
Conversation persistence and compaction are defined in
[docs/conversations.md](docs/conversations.md).
Provider and model configuration are defined in [docs/models.md](docs/models.md).
The intentionally small Team model is defined in [docs/teams.md](docs/teams.md).
Dispatch, Run, and provider-attempt boundaries are defined in
[docs/runs.md](docs/runs.md).
Topic Documents, direct edits, and the main revision line are defined in
[docs/documents.md](docs/documents.md).
Proposal authorship, review, and human-only merge are defined in
[docs/proposals.md](docs/proposals.md).
Comment and formal Review semantics are defined in
[docs/comments-and-reviews.md](docs/comments-and-reviews.md).
The relational entities and invariants are defined in
[docs/core-data-model.md](docs/core-data-model.md), with the first HTTP surface
in [docs/http-api.md](docs/http-api.md).
The executable technology and deployment shape are recorded in
[docs/architecture.md](docs/architecture.md).

## Status

The project is in the product, protocol, and executable-foundation design phase.
The first-version implementation stack is a Rust modular monolith with bundled
SQLite. The initial dependency lock exists; the open-source license still needs
to be selected. The project is not being designed as a commercial hosted
service.

The executable foundation currently includes the CLI runtime roles, embedded
SQLite migrations, initial domain state machines, Human-only merge permission,
and an HTTP health endpoint. Issue and Proposal use-case endpoints are not
implemented yet.

## Run the foundation

Rust 1.94 or newer is required.

```bash
cargo run -- bootstrap --handle admin --display-name "Administrator"
cargo run -- dev
```

Bootstrap prints the only copy of the first Human Member's bearer token. Store
it securely; subsequent bootstrap attempts are rejected and Synod stores only
the token digest.

The default server listens on `127.0.0.1:3030` and creates `synod.db` in the
working directory. Check it with:

```bash
curl http://127.0.0.1:3030/api/v1/health
```

Authenticated endpoints use that token:

```bash
curl -H "Authorization: Bearer $SYNOD_TOKEN" \
  http://127.0.0.1:3030/api/v1/me
```

Run the worker storage check without starting its loop:

```bash
cargo run -- worker --once
```

Validate the repository with:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```
