use std::future::Future;

use serde::Serialize;

use crate::domain::{ModelRequest, ModelResponse, ProviderAdapter};

mod http;

pub use http::{HttpGateway, validate_provider_endpoint};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiscoveredModel {
    pub id: String,
    pub owned_by: Option<String>,
}

#[derive(Clone)]
pub struct ProviderRoute {
    pub adapter: ProviderAdapter,
    pub base_url: String,
    pub credential_ref: String,
    pub model_name: String,
    pub defaults: serde_json::Value,
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("provider adapter is not supported: {0}")]
    UnsupportedAdapter(String),
    #[error("provider endpoint is not allowed: {0}")]
    Endpoint(String),
    #[error("provider credential is unavailable: {0}")]
    Credential(String),
    #[error("provider request failed: {0}")]
    Request(String),
    #[error("provider response was invalid: {0}")]
    InvalidResponse(String),
}

pub trait ModelGateway: Send + Sync {
    fn complete(
        &self,
        route: ProviderRoute,
        request: ModelRequest,
    ) -> impl Future<Output = Result<ModelResponse, ProviderError>> + Send;
}
