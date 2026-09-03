# Releasing Synod

Synod has no stable release yet. Do not tag a release merely because the
manifest contains a version.

## Before the first public push

1. Confirm that every commit uses an author email the maintainer is willing to
   expose publicly. Rewrite unpublished history if privacy requires it.
2. Re-run the secret scan over both the working tree and Git history.
3. Verify the remote is the intended public repository and contains no
   unrelated history.
4. Push `main`, then wait for the `CI` workflow to pass.
5. Enable private vulnerability reporting and require the CI check on protected
   branches or an equivalent repository ruleset.
6. Add a concise repository description and relevant GitHub Topics.

## Release validation

Run the same checks as CI:

```bash
npm --prefix web ci
npm --prefix web run check
npm --prefix web run build
git diff --exit-code -- web/dist
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Then:

1. Update `CHANGELOG.md` and remove claims that are not backed by executable
   behavior.
2. Confirm `Cargo.toml`, `Cargo.lock`, and the embedded Web assets agree with the
   intended version.
3. Create an annotated `vX.Y.Z` tag from a clean, CI-verified commit.
4. Create a GitHub Release from that tag and include known limitations.
5. Verify the release page and archive contents from an unauthenticated view.

Synod currently has `publish = false`; a GitHub Release does not imply crates.io
publication.
