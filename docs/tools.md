# AI member tools

## Boundary

Synod AI members may inspect project material but cannot execute or modify it.
Tools are built into Synod and operate only on immutable WorkspaceSnapshots and
durable Synod objects.

```text
AI member model
      |
      | normalized tool call
      v
Synod read-tool broker
      |
      +--> WorkspaceSnapshot
      +--> Issue / Proposal / Artifact
```

This is a context exploration facility, not a coding-agent runtime.

## First-version tools

### `list_files`

```yaml
snapshot_id: WS-7
path: src
glob: "**/*.py"
max_depth: 4
```

Returns normalized relative paths and basic metadata. Results are sorted and
bounded.

### `read_file`

```yaml
snapshot_id: WS-7
path: src/auth/token.py
start_line: 1
end_line: 240
```

Returns a line-numbered text range. Binary files are rejected unless a later
media adapter explicitly supports their type.

### `search_text`

```yaml
snapshot_id: WS-7
query: verify_token
path: src
glob: "**/*.py"
max_results: 50
```

Returns bounded matches with file path, line number, and a short excerpt.

### `read_reference`

```yaml
reference: "#12"
```

Reads an accessible Issue, Proposal, or Artifact referenced from the current
Topic. Cross-Topic access requires explicit permission and a fully qualified
reference.

## WorkspaceSnapshots

A caller creates a snapshot from an explicit file selection:

```bash
synod issue attach TOP-1#20 \
  --path src/auth \
  --path pyproject.toml \
  --as workspace
```

The server stores a manifest and immutable file content:

```yaml
id: WS-7
topic_id: TOP-1
issue_id: 20
files:
  - path: src/auth/token.py
    size: 8412
    digest: sha256:...
created_by: caller-id
created_at: timestamp
```

Refreshing creates `WS-8`; it never changes `WS-7`. Runs and tool calls always
refer to an exact snapshot ID.

## Safety rules

- paths are normalized relative paths;
- absolute paths and `..` traversal are rejected;
- symlinks are resolved during snapshot creation and cannot escape the selected
  roots;
- special files, sockets, devices, and named pipes are rejected;
- archive extraction enforces file-count, path, and size limits;
- provider credentials and Synod configuration are never part of a snapshot;
- file content is untrusted data, not model instruction authority.

## Budgets

Each Run has an explicit tool budget:

```yaml
max_calls: 20
max_total_bytes: 500000
max_tool_output_tokens: 20000
max_lines_per_read: 500
```

The broker rejects calls after a limit is exhausted. Repeated identical calls
may be served from cache but still appear in the audit log.

## Audit log

Every call records:

```yaml
run_id: RUN-42
ai_member: security-reviewer
tool: search_text
arguments_digest: sha256:...
snapshot_id: WS-7
started_at: timestamp
completed_at: timestamp
result_count: 6
result_bytes: 4812
outcome: success
```

Sensitive arguments may be redacted in presentation, but the system must not
claim reproducibility if required audit data was deliberately omitted.

## Provider compatibility

Model declares capabilities such as:

```yaml
tool_calling: true
structured_output: true
streaming: true
```

Providers with tool calling use a bounded `model -> tool -> model` loop.
Providers without it receive the preassembled context pack and cannot invoke
read tools in that Run. Provider adapters normalize calls into the same internal
tool request and result contracts.

## Explicit exclusions

These tools are outside Synod's core boundary:

```text
write_file
execute_shell
run_tests
git_commit
network_request
install_package
arbitrary MCP tools
```

If an external orchestrator needs those operations, it performs them locally and
posts resulting evidence or a new WorkspaceSnapshot back to Synod.
