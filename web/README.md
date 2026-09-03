# Synod Web

The Web UI is a Svelte 5 single-page application for the local Synod process.
It has no application server, cloud workspace, tunnel, analytics, CDN, or remote
font dependency.

The current product slice supports the complete path from configuring a
DeepSeek or MiniMax route to creating an AI Member, seating it in a Topic,
opening an Issue with an `@mention`, and reading its durable response in the
Issue timeline. Settings offers local API-key storage for the simplest setup or
an environment-variable reference for externally managed credentials. Secret
values are accepted write-only and never returned by the API.
Each saved Provider also exposes a `Test + models` action. Discovery runs in the
Rust process with the stored credential; choosing a result fills the separate
Model form rather than silently changing configuration.

For frontend development, run the Rust API and Vite separately:

```bash
cargo run -- dev
npm --prefix web run dev
```

Vite listens on `127.0.0.1:5173` and proxies `/api` to
`127.0.0.1:3030`. Production assets are deterministic and embedded into the
Rust binary:

```bash
npm --prefix web run check
npm --prefix web run build
cargo build --release
```

Commit changes under `web/dist/` together with their source changes. This keeps
Rust-only builds and end-user installations independent from Node.
