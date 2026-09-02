# Providers and models

## Principle

Model access is explicit and simple. Synod does not need a model router in its
first version.

```text
Provider -> Model -> AI Member -> Conversation
```

- Provider describes how and where to call an API.
- Model identifies one configured model and its capabilities.
- AI Member adds an identity Prompt and chooses a default Model.
- Conversation belongs to an AI Member and an Issue or Proposal, but remains
  provider-neutral.

## Provider

```yaml
id: provider-anthropic
name: Anthropic
adapter: anthropic_messages
base_url: https://api.anthropic.com
credential_ref: secret://anthropic/default
enabled: true
```

Initial adapter families:

```text
openai_responses
openai_compatible
anthropic_messages
google_gemini
```

Local and gateway services use the nearest compatible adapter plus a custom base
URL. Additional native adapters are added only when protocol differences cannot
be represented safely by an existing one.

Credentials are referenced from a server-side secret source and are never
returned through the API, inserted into prompts, or stored in WorkspaceSnapshots.
The first implementation accepts only `env://` and `secret://` credential
references and returns a boolean `credential_configured` field instead of the
reference value.

## Model

```yaml
id: claude-sonnet
provider_id: provider-anthropic
model_name: configured-model-name
display_name: Claude Sonnet
capabilities:
  streaming: true
  tool_calling: true
  structured_output: false
  vision: true
  provider_conversation: false
limits:
  context_tokens: configured-value
  max_output_tokens: configured-value
defaults:
  temperature: null
  max_output_tokens: 8000
enabled: true
```

Capability declarations are configuration validated by adapter behavior. Synod
does not assume all vendors interpret similarly named parameters in the same
way.

If a Run requires an unsupported capability, admission fails before spending
tokens. The system does not silently remove tools, change the output contract, or
substitute another model.

## AI Member

```yaml
id: member-architect
kind: ai
handle: architect
display_name: Architect
identity_prompt: |
  Review boundaries, scalability, migration risk, and unnecessary complexity.
identity_prompt_version: 3
default_model_id: claude-sonnet
enabled: true
```

Identity is only the Prompt. It does not select special application code,
permissions, or tools. All AI Members run through the same engine.

Changing the default Model affects later Runs only. Every Run records the exact
Prompt version, Provider, Model, parameters, and capabilities used.

## Provider adapter contract

The workflow engine owns the model/tool loop. An adapter translates one provider
turn into normalized events:

```text
start_turn(request) -> stream<event>
cancel_turn(provider_request_id)
count_or_estimate_tokens(request)
```

Normalized events include:

```text
response_started
text_delta
tool_call_started
tool_call_arguments_delta
tool_call_completed
usage
response_completed
response_failed
```

The adapter does not decide which Synod tool to execute, how to authorize it, or
whether another model turn is allowed. Those policies remain provider-neutral in
the workflow engine.

## No automatic fallback in the first version

Automatic fallback sounds convenient but weakens reproducibility and identity
consistency. A Run that starts with one Model should not silently finish with
another.

On failure:

```text
retry same Model
or
explicitly retry with another Model as a new Run
```

The new Run links to the failed Run and records who requested the substitution.

## Switching models in a Conversation

A user may change an AI Member's Model while continuing its Conversation. Synod
then rebuilds active context from its canonical transcript and context epoch.
Provider-specific cursors are ignored unless they match the selected Model.

The timeline visibly records the boundary:

```text
Run 41: architect using Model A
Run 42: architect changed to Model B
```

The identity Prompt remains the same unless separately edited.

## First-version scope

- four adapter families;
- explicit Providers and Models;
- one default Model per AI Member;
- per-Run Model override;
- capability validation before invocation;
- normalized streaming and tool-call events;
- retry on the same Model;
- explicit cross-model retry as a new Run;
- no load balancing, scoring, automatic fallback, or dynamic routing.
