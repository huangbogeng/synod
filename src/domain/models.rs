use serde::{Deserialize, Serialize};

use super::{MembershipRole, ModelId, Principal, PrincipalId, ProviderId, TeamId, TopicId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderAdapter {
    OpenaiResponses,
    OpenaiCompatible,
    AnthropicMessages,
    GoogleGemini,
}

impl ProviderAdapter {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OpenaiResponses => "openai_responses",
            Self::OpenaiCompatible => "openai_compatible",
            Self::AnthropicMessages => "anthropic_messages",
            Self::GoogleGemini => "google_gemini",
        }
    }

    pub fn from_stored(value: &str) -> Option<Self> {
        match value {
            "openai_responses" => Some(Self::OpenaiResponses),
            "openai_compatible" => Some(Self::OpenaiCompatible),
            "anthropic_messages" => Some(Self::AnthropicMessages),
            "google_gemini" => Some(Self::GoogleGemini),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provider {
    pub id: ProviderId,
    pub name: String,
    pub adapter: ProviderAdapter,
    pub base_url: String,
    pub credential_configured: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Model {
    pub id: ModelId,
    pub provider_id: ProviderId,
    pub model_name: String,
    pub display_name: String,
    pub capabilities: serde_json::Value,
    pub limits: serde_json::Value,
    pub defaults: serde_json::Value,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelInput {
    pub provider_id: ProviderId,
    pub model_name: String,
    pub display_name: String,
    pub capabilities: serde_json::Value,
    pub limits: serde_json::Value,
    pub defaults: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiMember {
    #[serde(flatten)]
    pub principal: Principal,
    pub identity_prompt_version: i64,
    pub default_model_id: ModelId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicMember {
    #[serde(flatten)]
    pub principal: Principal,
    pub role: MembershipRole,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Team {
    pub id: TeamId,
    pub topic_id: TopicId,
    pub handle: String,
    pub display_name: String,
    pub members: Vec<PrincipalId>,
}
