# Security policy

Synod is pre-1.0 software intended for trusted local deployment. It stores
model-provider credentials and issues authenticated outbound requests, so do
not expose the server directly to an untrusted network.

## Supported versions

Security fixes are applied to the latest commit on `main` and, after releases
begin, to the newest release only.

## Reporting a vulnerability

Use GitHub's private vulnerability reporting flow from the repository Security
tab. Do not include secrets or exploit details in a public Issue.

If private reporting is unavailable, open a minimal public Issue requesting a
private maintainer contact. Include no vulnerability details in that Issue.

Please describe the affected version, impact, reproduction conditions, and any
known mitigation. Maintainers will acknowledge a report as soon as practical
and coordinate disclosure after a fix is available.

## Current deployment boundary

- Synod binds to `127.0.0.1` by default and does not provide a tunnel.
- Provider credentials are write-only through the HTTP API.
- Local credentials are stored in the SQLite database; protect the database
  file and host account accordingly.
- AI Members never receive bearer credentials.
- Only Human principals may authorize merges.
