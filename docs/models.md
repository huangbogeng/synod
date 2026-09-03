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

Identity is only the Prompt. It does not select special application code,
permissions, or tools. All AI Members run through the same engine. Changing the
default Model affects later Runs only; each Run freezes the selected Provider,
Model, parameters, identity Prompt version, and context.

## Implemented providers

The first HTTP implementation intentionally supports only DeepSeek and MiniMax.
Both expose an OpenAI-compatible `POST /chat/completions` API with Bearer-token
authentication, so they share a small wire adapter rather than two vendor SDKs.
Vendor differences remain explicit in endpoint validation, allowed parameters,
and response error handling.

The Web UI can test a saved Provider and discover its models without returning
the credential to the browser. Synod calls the vendor's authenticated model-list
endpoint with a 15-second timeout, bounds the response to 2 MiB and 1,000 usable
model identifiers, then sorts and deduplicates it. The selected identifier is
copied into the explicit Model form; discovery never creates or silently
switches a Model.

### DeepSeek

```json
{
  "name": "DeepSeek",
  "adapter": "openai_compatible",
  "base_url": "https://api.deepseek.com",
  "credential_ref": "env://DEEPSEEK_API_KEY",
  "enabled": true
}
```

`https://api.deepseek.com/v1` is also accepted. Configure the exact current
model identifier separately on the Model; Synod does not hard-code or silently
substitute a vendor model.

Allowed model defaults:

```text
max_tokens, temperature, top_p, thinking, reasoning_effort,
response_format, stop
```

Sources: [DeepSeek Chat Completions API](https://api-docs.deepseek.com/api/create-chat-completion/),
[DeepSeek List Models](https://api-docs.deepseek.com/api/list-models),
[official curl example](https://api-docs.deepseek.com/api_samples/chat_curl), and
[error codes](https://api-docs.deepseek.com/quick_start/error_codes/).

### MiniMax

International endpoint:

```json
{
  "name": "MiniMax",
  "adapter": "openai_compatible",
  "base_url": "https://api.minimax.io/v1",
  "credential_ref": "env://MINIMAX_API_KEY",
  "enabled": true
}
```

For the mainland China service, use `https://api.minimaxi.com/v1`. Synod uses
the current OpenAI-compatible endpoint, not the deprecated
`/v1/text/chatcompletion_v2` API. It defaults `reasoning_split` to `true`, so
reasoning content remains separate from the final answer.

Allowed model defaults:

```text
max_completion_tokens, temperature, top_p, thinking, reasoning_split,
service_tier
```

MiniMax may return HTTP success with a non-zero `base_resp.status_code`; Synod
treats that as a failed provider attempt.

Sources: [MiniMax OpenAI-compatible Chat Completions](https://platform.minimax.io/docs/api-reference/text-chat-openai),
[MiniMax List Models](https://platform.minimax.io/docs/api-reference/models/openai/list-models),
[model invocation guide](https://platform.minimax.io/docs/guides/text-generation),
and [error codes](https://platform.minimax.io/docs/api-reference/errorcode).

## Credential boundary

The preset-first Web form offers two local credential modes:

- a write-only `api_key`, stored in `provider_secrets` inside the local SQLite
  database and represented internally by `secret://<provider-id>`;
- an `env://NAME` reference, resolved from the worker process environment.

The database file is created with owner-only permissions on Unix. This is local
at-rest access control, not encryption: users requiring an external secret
manager should choose `env://NAME`. In both modes, public Provider JSON returns
only `credential_configured`; credential references and secret values are never
returned or inserted into model context. Environment variable names accept only
ASCII letters, digits, and underscores.

## Safety and reproducibility

- only HTTPS and the three documented vendor hosts are allowed;
- URL credentials, custom ports, arbitrary paths, streaming, and request-level
  tools are rejected or excluded;
- configured defaults are copied only from the vendor-specific allowlist and
  cannot replace `model`, `messages`, or `stream`;
- response bodies are bounded to 2 MiB;
- there is no automatic model fallback or provider substitution;
- a failed call records a failed Run without publishing a fake AI Comment.

The adapter is deliberately non-streaming and single-turn today. Conversation
history, read-only tools, multi-turn tool loops, cancellation, and retry policy
will be added at the provider-neutral workflow layer rather than hidden in a
vendor integration.

## Model and AI Member examples

```yaml
model:
  provider_id: configured-provider-id
  model_name: exact-vendor-model-id
  display_name: Review Model
  capabilities:
    streaming: false
    tool_calling: false
    structured_output: false
    vision: false
    provider_conversation: false
  defaults:
    temperature: 0.2

ai_member:
  handle: architect
  display_name: Architect
  identity_prompt: |
    Review boundaries, scalability, migration risk, and unnecessary complexity.
  default_model_id: configured-model-id
  enabled: true
```

If a later Run needs another Model, changing the configured default affects only
new Runs. Cross-model retry must create a new Run and remain visible in history.
