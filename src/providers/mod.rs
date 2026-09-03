use std::future::Future;

use crate::domain::{ModelRequest, ModelResponse, ProviderAdapter};

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
