use std::{future::Future, time::Duration};

use reqwest::{Client, StatusCode, Url, redirect::Policy};
use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::domain::{ModelRequest, ModelResponse, ProviderAdapter};
use crate::persistence::Database;

use super::{DiscoveredModel, ModelGateway, ProviderError, ProviderRoute};

const MAX_RESPONSE_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Vendor {
    DeepSeek,
    MiniMax,
}

#[derive(Clone)]
pub struct HttpGateway {
    client: Client,
    database: Database,
}

impl HttpGateway {
    pub fn new(database: Database) -> Result<Self, ProviderError> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(270))
            .redirect(Policy::none())
            .user_agent(concat!("synod/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|error| ProviderError::Request(error.to_string()))?;
        Ok(Self { client, database })
    }

    pub async fn discover_models(
        &self,
        base_url: &str,
        credential_ref: &str,
    ) -> Result<Vec<DiscoveredModel>, ProviderError> {
        let endpoint = models_endpoint(base_url)?;
        let credential = resolve_credential(&self.database, credential_ref).await?;
        let response = self
            .client
            .get(endpoint)
            .bearer_auth(credential)
            .timeout(Duration::from_secs(15))
            .send()
            .await
            .map_err(|error| ProviderError::Request(error.to_string()))?;
        let status = response.status();
        let bytes = read_bounded(response).await?;
        if !status.is_success() {
            return Err(http_error(status, &bytes));
        }
        decode_models(&bytes)
    }
}

impl ModelGateway for HttpGateway {
    fn complete(
        &self,
        route: ProviderRoute,
        request: ModelRequest,
    ) -> impl Future<Output = Result<ModelResponse, ProviderError>> + Send {
        let client = self.client.clone();
        let database = self.database.clone();
        async move { complete(&client, &database, route, request).await }
    }
}

async fn complete(
    client: &Client,
    database: &Database,
    route: ProviderRoute,
    request: ModelRequest,
) -> Result<ModelResponse, ProviderError> {
    validate_provider_endpoint(route.adapter, &route.base_url)?;
    let (endpoint, vendor) = endpoint(&route.base_url)?;
    let credential = resolve_credential(database, &route.credential_ref).await?;
    let payload = request_payload(vendor, &route, &request)?;
    let response = client
        .post(endpoint)
        .bearer_auth(credential)
        .json(&payload)
        .send()
        .await
        .map_err(|error| ProviderError::Request(error.to_string()))?;
    parse_response(response).await
}

pub fn validate_provider_endpoint(
    adapter: ProviderAdapter,
    base_url: &str,
) -> Result<(), ProviderError> {
    if adapter != ProviderAdapter::OpenaiCompatible {
        return Err(ProviderError::UnsupportedAdapter(
            adapter.as_str().to_owned(),
        ));
    }
    endpoint(base_url)?;
    Ok(())
}

fn endpoint(base_url: &str) -> Result<(Url, Vendor), ProviderError> {
    let mut url = Url::parse(base_url)
        .map_err(|_| ProviderError::Endpoint("base URL must be a valid absolute URL".to_owned()))?;
    if url.scheme() != "https" || url.username() != "" || url.password().is_some() {
        return Err(ProviderError::Endpoint(
            "only credential-free HTTPS base URLs are allowed".to_owned(),
        ));
    }
    let vendor = match url.host_str() {
        Some("api.deepseek.com") => Vendor::DeepSeek,
        Some("api.minimax.io" | "api.minimaxi.com") => Vendor::MiniMax,
        _ => {
            return Err(ProviderError::Endpoint(
                "only official DeepSeek and MiniMax API hosts are supported".to_owned(),
            ));
        }
    };
    if url.port().is_some() {
        return Err(ProviderError::Endpoint(
            "custom API ports are not allowed".to_owned(),
        ));
    }
    url.set_query(None);
    url.set_fragment(None);
    let path = url.path().trim_end_matches('/');
    let endpoint_path = match (vendor, path) {
        (Vendor::DeepSeek, "") => "/chat/completions",
        (Vendor::DeepSeek, "/v1") => "/v1/chat/completions",
        (Vendor::DeepSeek, "/chat/completions") => "/chat/completions",
        (Vendor::DeepSeek, "/v1/chat/completions") => "/v1/chat/completions",
        (Vendor::MiniMax, "/v1") => "/v1/chat/completions",
        (Vendor::MiniMax, "/v1/chat/completions") => "/v1/chat/completions",
        _ => {
            return Err(ProviderError::Endpoint(
                "base URL path is not a supported Chat Completions endpoint".to_owned(),
            ));
        }
    };
    url.set_path(endpoint_path);
    Ok((url, vendor))
}

fn models_endpoint(base_url: &str) -> Result<Url, ProviderError> {
    let (mut url, vendor) = endpoint(base_url)?;
    let path = match vendor {
        Vendor::DeepSeek if url.path().starts_with("/v1/") => "/v1/models",
        Vendor::DeepSeek => "/models",
        Vendor::MiniMax => "/v1/models",
    };
    url.set_path(path);
    Ok(url)
}

async fn resolve_credential(database: &Database, reference: &str) -> Result<String, ProviderError> {
    if let Some(name) = reference.strip_prefix("env://") {
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            return Err(ProviderError::Credential(
                "environment variable reference is invalid".to_owned(),
            ));
        }
        return std::env::var(name).map_err(|_| {
            ProviderError::Credential(format!("environment variable {name} is not set"))
        });
    }
    if reference.starts_with("secret://") {
        return database
            .resolve_provider_secret(reference)
            .await
            .map_err(|error| ProviderError::Credential(error.to_string()))?
            .ok_or_else(|| {
                ProviderError::Credential("local provider secret is missing".to_owned())
            });
    }
    Err(ProviderError::Credential(
        "credential reference scheme is unsupported".to_owned(),
    ))
}

fn request_payload(
    vendor: Vendor,
    route: &ProviderRoute,
    request: &ModelRequest,
) -> Result<Value, ProviderError> {
    let context = serde_json::to_string_pretty(&request.context)
        .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
    let mut payload = Map::from_iter([
        ("model".to_owned(), Value::String(route.model_name.clone())),
        (
            "messages".to_owned(),
            json!([
                {"role": "system", "content": request.system_prompt},
                {"role": "user", "content": format!("Review the following Synod context and answer the trigger request.\n\n{context}")}
            ]),
        ),
        ("stream".to_owned(), Value::Bool(false)),
    ]);
    let allowed: &[&str] = match vendor {
        Vendor::DeepSeek => &[
            "max_tokens",
            "temperature",
            "top_p",
            "thinking",
            "reasoning_effort",
            "response_format",
            "stop",
        ],
        Vendor::MiniMax => &[
            "max_completion_tokens",
            "temperature",
            "top_p",
            "thinking",
            "reasoning_split",
            "service_tier",
        ],
    };
    let defaults = route.defaults.as_object().ok_or_else(|| {
        ProviderError::InvalidResponse("Model defaults must be a JSON object".to_owned())
    })?;
    for key in allowed {
        if let Some(value) = defaults.get(*key) {
            payload.insert((*key).to_owned(), value.clone());
        }
    }
    if vendor == Vendor::MiniMax && !payload.contains_key("reasoning_split") {
        payload.insert("reasoning_split".to_owned(), Value::Bool(true));
    }
    Ok(Value::Object(payload))
}

async fn parse_response(response: reqwest::Response) -> Result<ModelResponse, ProviderError> {
    let status = response.status();
    let bytes = read_bounded(response).await?;
    if !status.is_success() {
        return Err(http_error(status, &bytes));
    }
    decode_success(&bytes)
}

async fn read_bounded(mut response: reqwest::Response) -> Result<Vec<u8>, ProviderError> {
    if response
        .content_length()
        .is_some_and(|size| size > MAX_RESPONSE_BYTES)
    {
        return Err(ProviderError::InvalidResponse(
            "response exceeds the 2 MiB limit".to_owned(),
        ));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| ProviderError::Request(error.to_string()))?
    {
        let new_len = bytes.len().saturating_add(chunk.len());
        if u64::try_from(new_len).unwrap_or(u64::MAX) > MAX_RESPONSE_BYTES {
            return Err(ProviderError::InvalidResponse(
                "response exceeds the 2 MiB limit".to_owned(),
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    id: String,
    owned_by: Option<String>,
}

fn decode_models(bytes: &[u8]) -> Result<Vec<DiscoveredModel>, ProviderError> {
    let response: ModelsResponse = serde_json::from_slice(bytes)
        .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
    let mut models = response
        .data
        .into_iter()
        .take(1_000)
        .filter(|model| !model.id.trim().is_empty() && model.id.chars().count() <= 200)
        .map(|model| DiscoveredModel {
            id: model.id,
            owned_by: model.owned_by,
        })
        .collect::<Vec<_>>();
    models.sort_by(|left, right| left.id.cmp(&right.id));
    models.dedup_by(|left, right| left.id == right.id);
    if models.is_empty() {
        return Err(ProviderError::InvalidResponse(
            "provider returned no usable models".to_owned(),
        ));
    }
    Ok(models)
}

fn decode_success(bytes: &[u8]) -> Result<ModelResponse, ProviderError> {
    let wire: ChatCompletion = serde_json::from_slice(bytes)
        .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
    if let Some(base) = wire.base_resp
        && base.status_code != 0
    {
        return Err(ProviderError::Request(format!(
            "MiniMax API error {}: {}",
            base.status_code,
            bounded(&base.status_msg, 500)
        )));
    }
    let content = wire
        .choices
        .into_iter()
        .next()
        .and_then(|choice| choice.message.content)
        .ok_or_else(|| ProviderError::InvalidResponse("response has no text choice".to_owned()))?;
    Ok(ModelResponse {
        text: content,
        usage: wire.usage,
        provider_request_id: wire.id,
    })
}

fn http_error(status: StatusCode, body: &[u8]) -> ProviderError {
    let detail = serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|value| {
            value
                .pointer("/error/message")
                .or_else(|| value.get("message"))
                .or_else(|| value.pointer("/base_resp/status_msg"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| String::from_utf8_lossy(body).into_owned());
    ProviderError::Request(format!(
        "HTTP {}: {}",
        status.as_u16(),
        bounded(&detail, 500)
    ))
}

fn bounded(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

#[derive(Debug, Deserialize)]
struct ChatCompletion {
    id: Option<String>,
    #[serde(default)]
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Value,
    base_resp: Option<BaseResponse>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Message {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BaseResponse {
    status_code: i64,
    #[serde(default)]
    status_msg: String,
}

#[cfg(test)]
mod tests {
    use crate::domain::{
        ContextInput, ContextIssue, ContextTopic, ContextTrigger, RunId, TopicId, TopicItemId,
    };

    use super::*;

    #[test]
    fn only_official_deepseek_and_minimax_hosts_are_allowed() {
        assert_eq!(
            endpoint("https://api.deepseek.com").unwrap().0.as_str(),
            "https://api.deepseek.com/chat/completions"
        );
        assert_eq!(
            endpoint("https://api.minimax.io/v1").unwrap().0.as_str(),
            "https://api.minimax.io/v1/chat/completions"
        );
        assert_eq!(
            endpoint("https://api.minimaxi.com/v1").unwrap().1,
            Vendor::MiniMax
        );
        assert!(endpoint("https://example.com/v1").is_err());
        assert!(endpoint("http://api.deepseek.com").is_err());
        assert!(endpoint("https://api.minimax.io/internal").is_err());
        assert!(
            validate_provider_endpoint(
                ProviderAdapter::AnthropicMessages,
                "https://api.deepseek.com"
            )
            .is_err()
        );
        assert_eq!(
            models_endpoint("https://api.deepseek.com")
                .unwrap()
                .as_str(),
            "https://api.deepseek.com/models"
        );
        assert_eq!(
            models_endpoint("https://api.deepseek.com/v1")
                .unwrap()
                .as_str(),
            "https://api.deepseek.com/v1/models"
        );
        assert_eq!(
            models_endpoint("https://api.minimaxi.com/v1")
                .unwrap()
                .as_str(),
            "https://api.minimaxi.com/v1/models"
        );
    }

    #[test]
    fn vendor_defaults_are_filtered_and_cannot_override_core_fields() {
        let route = ProviderRoute {
            adapter: ProviderAdapter::OpenaiCompatible,
            base_url: "https://api.deepseek.com".to_owned(),
            credential_ref: "env://DEEPSEEK_API_KEY".to_owned(),
            model_name: "deepseek-v4-pro".to_owned(),
            defaults: json!({
                "max_tokens": 4096,
                "max_completion_tokens": 999,
                "model": "attacker-model",
                "stream": true
            }),
        };
        let payload = request_payload(Vendor::DeepSeek, &route, &request()).unwrap();
        assert_eq!(payload["model"], "deepseek-v4-pro");
        assert_eq!(payload["stream"], false);
        assert_eq!(payload["max_tokens"], 4096);
        assert!(payload.get("max_completion_tokens").is_none());

        let mut minimax_route = route;
        minimax_route.base_url = "https://api.minimax.io/v1".to_owned();
        minimax_route.model_name = "configured-minimax-model".to_owned();
        minimax_route.defaults = json!({});
        let payload = request_payload(Vendor::MiniMax, &minimax_route, &request()).unwrap();
        assert_eq!(payload["reasoning_split"], true);
    }

    #[test]
    fn common_response_shape_preserves_usage() {
        let response = decode_success(
            &serde_json::to_vec(&json!({
                "id": "response-1",
                "choices": [{"message": {"role": "assistant", "content": "Reviewed."}}],
                "usage": {"prompt_tokens": 10, "completion_tokens": 2},
                "base_resp": {"status_code": 0, "status_msg": ""}
            }))
            .unwrap(),
        )
        .unwrap();
        assert_eq!(response.text, "Reviewed.");
        assert_eq!(response.usage["prompt_tokens"], 10);
        assert_eq!(response.provider_request_id.as_deref(), Some("response-1"));
    }

    #[test]
    fn model_discovery_is_sorted_deduplicated_and_bounded() {
        let models = decode_models(
            serde_json::to_vec(&json!({
                "object": "list",
                "data": [
                    {"id": "model-b", "owned_by": "vendor"},
                    {"id": "model-a", "owned_by": "vendor"},
                    {"id": "model-b", "owned_by": "vendor"},
                    {"id": "", "owned_by": "vendor"}
                ]
            }))
            .unwrap()
            .as_slice(),
        )
        .unwrap();
        assert_eq!(
            models
                .iter()
                .map(|model| model.id.as_str())
                .collect::<Vec<_>>(),
            ["model-a", "model-b"]
        );
        assert!(decode_models(br#"{"data":[]}"#).is_err());
    }

    #[test]
    fn minimax_business_error_in_successful_http_body_is_rejected() {
        let error = decode_success(
            serde_json::to_vec(&json!({
                "choices": [],
                "base_resp": {"status_code": 1002, "status_msg": "rate limit"}
            }))
            .unwrap()
            .as_slice(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("MiniMax API error 1002"));
    }

    fn request() -> ModelRequest {
        ModelRequest {
            run_id: RunId::new(),
            context_snapshot_id: crate::domain::ContextSnapshotId::new(),
            system_prompt: "Review carefully.".to_owned(),
            context: ContextInput {
                topic: ContextTopic {
                    id: TopicId::new(),
                    title: "Synod".to_owned(),
                    description: String::new(),
                    revision: 1,
                },
                issue: ContextIssue {
                    id: TopicItemId::new(),
                    title: "Review".to_owned(),
                    issue_type: "code_audit".to_owned(),
                    state: "open".to_owned(),
                    body: "Inspect this.".to_owned(),
                    revision: 1,
                },
                trigger: ContextTrigger {
                    source_type: "issue".to_owned(),
                    source_id: "source".to_owned(),
                    source_revision: 1,
                    author_handle: "alice".to_owned(),
                    body: "Inspect this.".to_owned(),
                },
                timeline: Vec::new(),
            },
        }
    }
}
