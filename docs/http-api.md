# HTTP API shape

## Conventions

The first API is resource-oriented JSON under `/api/v1`. Web UI and CLI use the
same API; neither receives a privileged merge path.

- bearer authentication resolves a server-side Principal;
- request bodies never choose the acting Principal;
- `Idempotency-Key` is required for create, dispatch, retry, merge, and upload
  completion operations;
- mutable-resource writes include `expected_revision`;
- list endpoints use opaque cursor pagination;
- timestamps use RFC 3339 UTC;
- long model output streams through Server-Sent Events (SSE).

Local installation begins with `synod bootstrap`, which creates the first Human
Member and prints a bearer token once. The database stores only its SHA-256
digest. Bootstrap is rejected after the first successful call. AI Members never
receive bearer credentials.

Public references such as `TOP-1#12` are accepted at CLI and UI boundaries. API
payloads return both opaque IDs and display references.

## Topics and membership

```text
POST   /topics
GET    /topics
GET    /topics/{topic_id}
PATCH  /topics/{topic_id}

GET    /topics/{topic_id}/members
PUT    /topics/{topic_id}/members/{principal_id}
DELETE /topics/{topic_id}/members/{principal_id}

GET    /topics/{topic_id}/timeline
GET    /me
```

Membership mutation is Human-admin only. Removing membership does not erase
historical authorship.

## Issues and Comments

```text
GET    /issue-types
POST   /topics/{topic_id}/issues
GET    /topics/{topic_id}/issues
GET    /issues/{issue_id}
PATCH  /issues/{issue_id}
POST   /issues/{issue_id}/close
POST   /issues/{issue_id}/reopen

POST   /issues/{issue_id}/comments
POST   /proposals/{proposal_id}/comments
PATCH  /comments/{comment_id}
DELETE /comments/{comment_id}
```

Creating or editing mention-bearing content commits the content first and then
enqueues one Dispatch from that exact revision. The response returns the saved
resource plus `dispatch_id` when mentions were accepted.

The first implementation resolves `GET /issue-types`, Issue creation/list/read,
and Issue Comment creation/list. Mutation, close/reopen, Proposal Comments, and
Comment edit/tombstone endpoints remain part of the planned surface.

## Proposals and Reviews

```text
POST   /topics/{topic_id}/proposals
GET    /topics/{topic_id}/proposals
GET    /proposals/{proposal_id}
PATCH  /proposals/{proposal_id}
POST   /proposals/{proposal_id}/open
POST   /proposals/{proposal_id}/close
POST   /proposals/{proposal_id}/reopen

POST   /proposals/{proposal_id}/reviews
GET    /proposals/{proposal_id}/reviews
POST   /reviews/{review_id}/dismiss

POST   /proposals/{proposal_id}/merge
```

The merge request contains `expected_proposal_revision` and an optional change
message. The server derives the actor and rejects all non-Human principals even
if their ordinary role is `write`.

`409 Conflict` reports stale Document bases or Proposal revision. It returns
structured conflicting Document IDs but never attempts an automatic merge.

## Documents and revisions

```text
GET    /topics/{topic_id}/documents
POST   /topics/{topic_id}/documents
GET    /documents/{document_id}
PUT    /documents/{document_id}/content
POST   /documents/{document_id}/rename
POST   /documents/{document_id}/archive
POST   /documents/{document_id}/restore

GET    /topics/{topic_id}/revisions
GET    /topic-revisions/{revision_id}
GET    /topic-revisions/{revision_id}/diff
```

Document writes contain the base Document version and create one immutable
Topic revision. Editing drafts in a browser does not call these endpoints until
the user saves.

## Mentions, Dispatches, and Runs

Normal dispatch is automatic after content creation. Explicit operations are:

```text
GET    /dispatches/{dispatch_id}
POST   /dispatches/{dispatch_id}/retry
POST   /dispatches/{dispatch_id}/cancel

GET    /runs/{run_id}
GET    /topics/{topic_id}/runs
GET    /context-snapshots/{snapshot_id}
GET    /runs/{run_id}/events
POST   /runs/{run_id}/retry
POST   /runs/{run_id}/cancel
GET    /notifications
```

`GET /runs/{run_id}/events` is an SSE stream with resumable event IDs. The Issue
or Proposal timeline receives only durable Run state events and final Comments
or Reviews, not token deltas.

Retrying one Run creates a new Run. Retrying mention expansion creates a new
Dispatch. Neither endpoint mutates the old record.

The current implementation exposes Dispatch detail, Run detail, Topic-scoped
Run listing for the local board, immutable Context Snapshot detail, and the
acting Human's notifications. Retry, cancellation, Run events, and notification
read-state mutation remain planned.

## Artifacts and workspace snapshots

```text
POST   /topics/{topic_id}/artifacts
GET    /artifacts/{artifact_id}
GET    /artifacts/{artifact_id}/content

POST   /topics/{topic_id}/workspace-snapshots
POST   /workspace-snapshots/{snapshot_id}/files
POST   /workspace-snapshots/{snapshot_id}/complete
GET    /workspace-snapshots/{snapshot_id}
```

Uploads use bounded size, file count, and normalized path limits. A completed
snapshot is immutable.

## Model administration

```text
GET    /providers
POST   /providers
GET    /providers/{provider_id}/models
PATCH  /providers/{provider_id}
GET    /models
POST   /models
PATCH  /models/{model_id}
GET    /ai-members
POST   /ai-members
PATCH  /ai-members/{member_id}
GET    /topics/{topic_id}/teams
POST   /topics/{topic_id}/teams
PUT    /teams/{team_id}/members/{principal_id}
```

Provider, Model, and AI Member records are server administrative resources. API
responses expose credential presence but never references or secret values.

The current implementation supports listing and creating Providers, Models, and
AI Members. Only the bootstrap Human may use these server-wide endpoints.
Provider creation accepts exactly one of a write-only `api_key` or an `env://`
credential reference. A local API key is stored under an internal `secret://`
reference. Responses expose only `credential_configured`, never either input.
The Provider-scoped models endpoint uses that server-side credential to test the
official connection and returns a bounded, sorted list of model identifiers.

Topic Human writers may list or add Topic Members, create/list Teams, and add a
Topic Member to a Team. Team nesting and Caller/System membership are rejected.

## Errors

Errors use one envelope:

```json
{
  "error": {
    "code": "proposal_stale",
    "message": "Proposal was prepared against an older document version.",
    "details": {"document_ids": ["..."]},
    "request_id": "..."
  }
}
```

Stable error codes are part of the API contract. Initial mappings include:

- `400`: malformed input or invalid state transition;
- `401`: missing or invalid authentication;
- `403`: valid Principal lacks permission or is not Human for merge;
- `404`: resource absent or intentionally hidden by authorization;
- `409`: revision conflict or incompatible concurrent state transition;
- `422`: syntactically valid but semantically invalid content;
- `429`: caller or model budget limit;
- `503`: model provider temporarily unavailable.

## Minimal creation response

Commands should not need to poll several resources after posting content:

```json
{
  "data": {
    "id": "...",
    "ref": "TOP-1#12",
    "revision": 1
  },
  "dispatch": {
    "id": "...",
    "status": "pending",
    "run_ids": []
  }
}
```

The `dispatch` field is `null` when no active mention was present.

Repeating a request with the same `Idempotency-Key` and identical payload
returns the original result. Reusing the key with a different payload is
rejected as invalid input.
