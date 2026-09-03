# Changelog

All notable changes to Synod will be documented here. The project follows
[Semantic Versioning](https://semver.org/) once tagged releases begin.

## Unreleased

### Added

- Rust modular-monolith foundation with Axum, SQLx, and bundled SQLite.
- Human bootstrap authentication and Human-only merge authorization.
- Local CLI rotation for the bootstrap Human bearer token.
- Topic, Issue, Comment, mention Dispatch, Run, Team, and Council workflows.
- Provider-neutral execution through DeepSeek and MiniMax.
- Local Svelte Web UI with Provider management and an AI Member roster.
- AI Member execution defaults with immutable resolved parameters on each Run.
- JSON-first local configuration commands for Member setup and confirmed Topic
  clearing.

### Security

- Provider credentials are write-only through the API and excluded from API
  responses.
- Local runtime databases, environment files, and build outputs are excluded
  from source control where appropriate.
