# First-version architecture

## Decision

Synod starts as a modular Rust monolith with two runtime roles exposed by one
binary:

```text
synod server  -> HTTP API + embedded Svelte Web UI + SSE
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
Web application      Svelte 5 + TypeScript + Vite
Live updates         Server-Sent Events
CLI                   Clap
Tests                 cargo test + svelte-check + Vite build
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

The Web UI is a small Svelte single-page application because the Topic board,
Council presence, and streamed Run state are interaction-heavy. It uses a typed,
thin client over the existing HTTP API rather than introducing a second backend.
Vite emits stable JavaScript and CSS assets which Rust embeds at compile time.
Installed users need only the `synod` binary; Node is a contributor dependency,
not a runtime dependency. The UI loads no public CDN, cloud font, analytics, or
remote application resource.

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

The current worker resolves at most one pending Dispatch and executes at most
one queued Run per pass. Dispatch expansion and creation of notifications,
Conversations, queued Runs, and `run.execute` jobs are atomic. Run execution
uses a leased job, a frozen context snapshot, and transactional settlement.
`synod worker --once` performs one bounded pass and exits.

Redis, RabbitMQ, Celery, and Kubernetes are not required. A future deployment
may add a queue adapter only after database polling becomes a measured problem.

## Model adapters

Provider adapters are implemented with Reqwest directly against documented HTTP
protocols. The first executable slice contains one narrow OpenAI-compatible
Chat Completions adapter restricted to official DeepSeek and MiniMax hosts:

```text
DeepSeek  -> https://api.deepseek.com[/v1]/chat/completions
MiniMax   -> https://api.minimax.io/v1/chat/completions
          -> https://api.minimaxi.com/v1/chat/completions
```

The adapter currently makes non-streaming text requests; streaming and tool
calls remain later workflow slices. Synod does not use LiteLLM in its core
because provider routing, fallback, agent abstraction, and its larger dependency
surface overlap with responsibilities Synod deliberately keeps explicit.

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
  cli/             local administration and remote API commands
web/               Svelte source, typed API client, and Vite configuration
web/dist/          deterministic assets embedded by Rust
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

The first executable version uses the bootstrap Human Member's hashed bearer
token for both browser and API access. The Web UI retains the entered token only
in `sessionStorage`, so closing the browser tab clears it. A future local session
exchange may replace this without changing domain authorization.

AI Members never log in and receive no bearer credential. They act only through
an attributable Run. Therefore the merge endpoint can reject AI execution by
construction as well as through the Human-principal authorization rule.

OIDC, OAuth application installation, and enterprise identity integration are
later extensions, not core domain concepts.

## Local-only deployment

```text
one synod dev process
SQLite
local blob directory
embedded Web UI
```

Synod binds to `127.0.0.1:3030` by default. It has no tunnel, relay, cloud
workspace, device pairing, runtime registration, or inbound Provider callback.
DeepSeek and MiniMax calls are outbound HTTPS requests only. Binding another
address is an explicit operator action and is outside the default local security
boundary.

## Explicit non-choices

The first version does not use:

- microservices;
- a general agent framework;
- LiteLLM as the domain abstraction;
- a general-purpose ORM;
- Redis or a message broker;
- Celery or a general workflow scheduler;
- a frontend application server or server-side JavaScript runtime;
- cloud fonts, analytics, or CDN assets;
- tunneling, relay gateways, device pairing, or cloud workspaces;
- WebSockets;
- Docker as a requirement for local use.

These can be reconsidered from observed constraints, not added preemptively.
