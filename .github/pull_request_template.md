## What changed

Describe the user-visible outcome and the implementation boundary.

## Why

Link the Issue or explain the problem this change resolves.

## Validation

- [ ] `npm --prefix web run check`
- [ ] `npm --prefix web run build` when frontend source changed
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test --all-targets --all-features`
- [ ] Documentation and committed `web/dist` assets are updated where needed
- [ ] No credentials, tokens, local databases, or private prompts are included

## Compatibility and risk

Describe schema migrations, API compatibility, authorization impact, and
rollback considerations. Write `None` when they do not apply.
