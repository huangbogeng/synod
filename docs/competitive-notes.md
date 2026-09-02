# Competitive notes

This document records product ideas worth retaining without copying competitors'
full product boundaries.

Competitor repositories are product research references only. Synod does not
copy or vendor their code and does not derive its implementation from their
internal architecture. It independently implements the selected interaction
principles under its own open-source license.

It describes publicly observable product behavior, not implementation material.
Synod will not reuse or adapt competitor source code, schemas, prompts, APIs, or
UI components. All implementation and contracts are designed independently from
Synod's requirements.

## Multica

Useful ideas:

- a durable issue-like object connects intent, discussion, invocations, and result;
- invocation completion and decision completion are separate;
- local execution and server-side coordination have an explicit boundary;
- progress, retries, timeouts, token use, and failures are observable;
- human comments can cause another evaluation without losing history;
- one application model is accessible from Web UI and CLI;
- server-side history enables audit and recovery.

Not adopted:

- persistent coding-agent identities bound to CLI tools and runtimes;
- agent-machine registration and daemon scheduling;
- general team/project/issue management;
- repository mutation and software-delivery workflows;
- leader/member squads as the primary deliberation protocol.

Synod replaces these with a GitHub-shaped Topic, Issue, Proposal, Review, Comment,
and Run model backed by direct model-provider routes.

## Quorum and MCO

Useful ideas:

- CLI/MCP-friendly invocation by another agent;
- compact synthesis by default and full transcript on demand;
- machine-readable output and artifacts;
- explicit debate and synthesis modes.

Improvement for Synod:

- make Topic history and human direction durable rather than treating each call
  as an isolated command.

## AI Colosseum

Useful ideas:

- freeze identical context before independent review;
- bound discussion by depth, novelty, convergence, and budget;
- focus later rounds on explicit disputes;
- retain adopted and unresolved arguments;
- allow automatic, model, or human judgment.

Improvement for Synod:

- treat the result as advice returned to an external executor, not as a winning
  model or autonomous implementation plan.

## General lesson

The defensible product is not a multi-model chat room. It is a familiar
Issue-to-Proposal workflow with durable provenance and a clean handoff back to
the caller.
