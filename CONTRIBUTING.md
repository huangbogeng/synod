# Contributing to Synod

Thank you for helping make model-assisted deliberation simpler and more
auditable.

Synod is pre-1.0. Before proposing a large implementation, open an Issue that
describes the user problem, the intended boundary, and the smallest useful
vertical slice. Bug fixes and focused documentation improvements may go
directly to a pull request.

## Development setup

Synod requires Rust 1.94 or newer and Node.js 24 or newer.

```bash
npm --prefix web ci
npm --prefix web run check
npm --prefix web run build
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

The generated `web/dist` assets are committed because the Rust binary embeds
them. Include the rebuilt assets whenever frontend source changes.

For a local run:

```bash
cargo run -- bootstrap --handle admin --display-name "Administrator"
cargo run -- dev
```

Never commit the printed bootstrap token, API keys, `.env` files, or local
SQLite databases.

## Pull requests

- Keep each pull request focused on one coherent change.
- Add or update tests for behavior and authorization boundaries.
- Update relevant documentation when a public contract changes.
- Preserve Human-only merge authorization in server-side code.
- Explain migrations and compatibility impact for schema changes.
- Do not silently shorten limits, weaken validation, or hide failed model Runs.

## Independent implementation

Synod studies other products for publicly observable interaction patterns, but
does not copy or adapt their source code, prompts, schemas, or private APIs.
Contributions must be original or clearly identify compatible third-party code
and its license in the pull request.
