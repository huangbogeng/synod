# First-version architecture

## Decision

Synod starts as a modular Rust monolith with two runtime roles exposed by one
binary:

```text
synod server  -> HTTP API + server-rendered Web UI + SSE
synod worker  -> durable Dispatch, Run, and provider execution
```

For local development, `synod dev` may start both roles in one process. Durable
work is still committed to the database before execution, so a process crash
does not erase accepted Runs.

## Technology choices

```text
Language/runtime     stable Rust + Tokio
HTTP/API             Axum + Tower middleware
Serialization        Serde + domain newtypes
Persistence          SQLx
Migrations           embedded SQLx migrations
First-version DB     bundled SQLite
Future database      PostgreSQL through a separate backend
Provider HTTP        Reqwest
Web rendering        Askama templates + vendored HTMX
Live updates         Server-Sent Events
CLI                   Clap
Tests                 cargo test
Packaging             one Cargo package and one synod binary
```

Exact dependency versions are recorded in `Cargo.lock`, not embedded into the
product protocol. The initial minimum supported Rust version is 1.94, matching
the selected SQLx 0.9 line.

## Why this shape

Axum uses the Tokio and Tower ecosystem without imposing a hidden application
model. Serde request types stay at the transport boundary while domain newtypes
and enums express identifiers, states, and allowed transitions explicitly.
SQLx keeps SQL and transactions visible and embeds migrations into the binary.

Bundled SQLite gives a zero-service installation for individuals and small
trusted teams without depending on the host's SQLite installation. PostgreSQL
is a future deployment path when multiple server or worker processes are
required. The first version does not carry a runtime database abstraction or
promise transparent database switching.

The Web UI is mostly forms, timelines, diffs, status updates, and streamed Run
output. Compile-time Askama templates plus HTMX cover that interaction without a
second TypeScript application, duplicated DTOs, or a mandatory Node toolchain.
Static browser dependencies are vendored so an installed Synod server does not
depend on a public CDN.

SSE is sufficient because user actions remain ordinary HTTP requests and live
updates flow from server to browser. WebSockets add no necessary first-version
capability.

## Durable jobs without queue infrastructure

Detached Tokio tasks are not used as the durable boundary for model Runs. They
belong to one process and disappear when that process exits.

Synod stores jobs in relational tables:

```text
queued -> leased -> completed
            |
            +-> queued after expired lease
            +-> completed with terminal failure
```

A worker claims bounded batches with a lease token and expiry. Heartbeats extend
active leases. Terminal Run state and the final Comment or Review are committed
transactionally.

The first version permits one active worker. A future PostgreSQL backend may
permit several workers using row locking. This is a small internal queue, not a
general workflow engine.

Redis, RabbitMQ, Celery, and Kubernetes are not required. A future deployment
may add a queue adapter only after database polling becomes a measured problem.

## Model adapters

Provider adapters are implemented with Reqwest directly against documented HTTP
protocols:

```text
openai_responses
openai_compatible
anthropic_messages
google_gemini
```

They share Synod's normalized stream and tool-call contract. Synod does not use
LiteLLM in its core because provider routing, fallback, agent abstraction, and
its larger dependency surface overlap with responsibilities Synod deliberately
keeps explicit.

Provider SDK crates may be used inside an adapter only when they materially
improve a native protocol implementation. No provider object may leak into
domain models, stored transcripts, or public API schemas.

## Module boundaries

```text
src/
  api/             Axum routes, extractors, auth, JSON schemas
  domain/          entities, newtypes, state transitions, authorization
  services/        use cases and transaction orchestration
  persistence/     SQLx repositories and transaction implementations
  workers/         job claiming and Run execution
  providers/       model protocol adapters
  tools/           authorized read-only tool broker
  web/             routes and template view models
  cli/             local administration and remote API commands
templates/         Askama HTML templates
static/            vendored HTMX and application assets
migrations/        embedded SQL migrations
```

Dependencies point inward:

```text
api / web / cli / workers
            |
            v
         services
            |
            v
          domain
```

Persistence and provider adapters implement ports used by services. Domain code
does not import Axum, SQLx, Reqwest, templates, or provider SDKs.

## Storage defaults

- bundled SQLite database and local content-addressed blob directory;
- PostgreSQL and S3-compatible blob storage as later deployment options;
- secret values from environment or a configured secret backend, never normal
  database columns;
- Markdown and small JSON stored directly; uploaded bytes stored by digest;
- size and retention limits configured at the Topic or server boundary.

## Authentication

The first executable version needs two credential paths:

- browser session for Human Members;
- hashed bearer tokens for external callers.

AI Members never log in and receive no bearer credential. They act only through
an attributable Run. Therefore the merge endpoint can reject AI execution by
construction as well as through the Human-principal authorization rule.

OIDC, OAuth application installation, and enterprise identity integration are
later extensions, not core domain concepts.

## Deployment profiles

### Local

```text
one synod dev process
SQLite
local blob directory
```

### Small server

```text
one synod server process
one synod worker process
SQLite on one host
local blob directory
```

### Multi-process later

```text
one or more servers
one or more workers
PostgreSQL
shared S3-compatible blob storage
```

The local profile is the primary product experience. The larger profile is not
part of the first-version compatibility promise and must not force
distributed-systems machinery into the core model.

## Explicit non-choices

The first version does not use:

- microservices;
- a general agent framework;
- LiteLLM as the domain abstraction;
- a general-purpose ORM;
- Redis or a message broker;
- Celery or a general workflow scheduler;
- a React/Vue SPA and separate frontend API client;
- WebSockets;
- Docker as a requirement for local use.

These can be reconsidered from observed constraints, not added preemptively.
