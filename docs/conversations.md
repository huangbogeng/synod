# Conversations

## Principle

An AI member is the same execution mechanism constrained by a versioned identity
prompt. Its continuity comes from a provider-neutral Conversation maintained by
Synod.

```text
AI member = identity Prompt + Model + Conversation + shared read tools
```

A Conversation belongs to one AI member and one discussion subject:

```text
(Issue | Proposal) x AI member
```

There is no hidden cross-Issue memory. Cross-Issue knowledge moves through Topic
knowledge, explicit references, merged Proposals, and Artifacts.

## Transcript and active context are different

Synod stores two related but distinct representations:

```text
Canonical transcript
  complete, append-only, inspectable
  messages, tool calls, tool results, Runs, and provenance

Active context projection
  bounded input sent to the next model call
  stable instructions + compacted history + recent verbatim tail
```

Compaction changes only the active projection. It never rewrites or deletes the
canonical transcript.

## Minimal storage model

```text
conversations
  id, subject_type, subject_id, ai_member_id, head_sequence

conversation_items
  conversation_id, sequence, kind, role, content_ref, run_id, created_at

context_epochs
  conversation_id, through_sequence, summary, summary_schema_version,
  model_id, identity_prompt_version, created_at

provider_cursors
  conversation_id, model_id, remote_conversation_id,
  previous_response_id, last_sequence
```

Conversation items are immutable after settlement. Editing an Issue Comment adds
a new revision event; it does not mutate the historical input of completed Runs.

## Conversation items

The item stream supports:

```text
human_message
model_message
tool_call
tool_result
context_note
compaction
error
```

Large tool results are stored as Artifacts. The item stream keeps a bounded
preview, digest, byte count, and Artifact reference instead of duplicating the
full output inside every future prompt.

## Provider independence

Provider-side conversation IDs and response chaining are optional acceleration,
not canonical state.

- If the same Provider and Model continue safely, Synod may use the remote
  cursor.
- If the Model changes or the remote state is unavailable, Synod rebuilds
  the request from its own active context projection.
- Provider cursors never replace local messages, tool calls, or provenance.
- A failed remote resume rebuilds from local state and is recorded in the Run.

This permits model switching without losing the visible conversation history.

## Compaction

Compaction begins before a provider context limit is reached. The threshold is a
policy based on estimated input tokens and reserved output/tool budget.

A context epoch contains a structured summary of older items:

```yaml
goal: string
confirmed_facts: []
accepted_constraints: []
decisions: []
open_questions: []
active_hypotheses: []
failed_approaches: []
artifact_refs: []
issue_and_proposal_refs: []
last_requested_action: string
```

The next active projection contains:

```text
current identity prompt
current Topic instructions
latest context-epoch summary
recent verbatim conversation tail
new trigger message
available read-tool definitions
```

Identity and Topic instructions are re-injected from their current authoritative
versions; they are not trusted to survive inside a generated summary.

## Safe boundaries

Compaction occurs only between provider turns, never while a tool call is
unsettled. A turn follows this order:

```text
persist input
  -> assemble projection
  -> call model
  -> settle all tool calls/results
  -> persist final model output
  -> optionally compact
```

New human messages arriving during a turn are queued. They enter the Conversation
at the next safe boundary rather than altering an in-flight model request.

## Concurrency

One Conversation has a single active writer. Additional triggers queue behind the
current turn. Different AI members on the same Issue use different Conversations
and may run in parallel.

```text
Issue #12
├── architect Conversation       -> serial internally
├── security Conversation        -> serial internally
└── quant-research Conversation  -> serial internally
```

This prevents two responses from racing to advance the same provider cursor.

## Continue, resume, and fork

- `continue`: append to the existing Conversation.
- `resume`: load an inactive Conversation and continue it.
- `fork`: copy its active projection and references into a new Conversation while
  leaving the original unchanged.
- `reset`: start a new Conversation for the same AI member and subject; retain the
  prior Conversation for audit.

A direct reply to an AI member's Comment continues that member's Conversation.
An explicit `@mention` also continues it unless the author selects fork or reset.

## First-version scope

Implement locally:

- append-only provider-neutral items;
- one Conversation per subject and AI member;
- optional provider cursor metadata;
- structured compaction summary plus recent raw tail;
- separate storage for large tool outputs;
- single-writer queue per Conversation;
- full transcript inspection.

The current execution slice persists the trigger and terminal model message or
error as ordered `conversation_items`. Context epochs, provider cursors, tool
turns, and compaction remain future layers.

Do not initially implement:

- cross-Issue private memory;
- deletion of raw transcript after compaction;
- provider-specific opaque compaction as the only resumable state;
- asynchronous changes to an in-flight context;
- full event sourcing of every transient UI update.
