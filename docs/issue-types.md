# Issue types

## Purpose

Issue types let one Topic contain software development, review, research, and
decision work without creating separate collaboration systems for each domain.

Every Issue uses the same identity, timeline, relationship, permission, and
`open | closed` lifecycle. Its type changes the default form and workflow only.

## Default types

### `task`

Minimal fields:

- objective;
- context;
- acceptance criteria.

### `bug`

Suggested fields:

- observed behavior;
- expected behavior;
- reproduction or evidence;
- affected scope;
- severity;
- acceptance criteria.

### `feature`

Suggested fields:

- user problem;
- proposed behavior;
- alternatives;
- constraints;
- acceptance criteria;
- non-goals.

### `code_audit`

Suggested fields:

- target and scope;
- audit dimensions;
- threat model or correctness contract;
- excluded areas;
- evidence requirements;
- severity rubric.

An audit normally produces Comments summarizing coverage and child Issues for
individual findings. It should not collapse unrelated findings into a single
unreviewable result.

### `research`

Suggested fields:

- research question;
- hypothesis or candidate explanations;
- existing evidence;
- required evidence;
- scope and constraints;
- stop conditions.

### `experiment`

Suggested fields:

- hypothesis under test;
- frozen protocol;
- inputs and sample;
- metrics and gates;
- execution environment;
- expected artifacts;
- interpretation rules.

### `decision`

Suggested fields:

- decision to make;
- options;
- constraints;
- evaluation criteria;
- deadline;
- decision owner.

## Type definition

A custom type is configuration rather than application code:

```yaml
slug: model_review
name: Model Review
description: Review a trained model before promotion
template: templates/model-review.md
suggested_team: model-governance
default_workflow: independent-review
proposal_template: templates/model-review-proposal.md
```

The initial implementation should avoid arbitrary executable hooks in type
definitions. Automation refers to named, server-owned workflows so importing a
type cannot introduce code execution.

## Changing type

Changing an Issue's type is allowed while preserving its number, Comments,
Runs, links, and history. The change adds a timeline event. Existing content is
never discarded merely because it does not match the new template.

## First-version decision

The first version exposes only the seven built-in types above. They are global,
seeded by a database migration, and use Markdown guidance rather than required
structured fields. None receives a specialized lifecycle or UI, and templates
do not trigger suggested mentions automatically.

Topic-scoped custom types remain a later extension. Their configuration shape is
documented above so the core Issue record does not need redesign when they are
added.
