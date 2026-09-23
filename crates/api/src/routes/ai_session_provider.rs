//! Process-memory AI provider configuration for the local Advanced Lab.

use std::{sync::Arc, time::Duration};

use ai_client::{
    AiApiProtocol, AiClientError, AiConfig, AiProvider, AiProviderCapabilities, AiProviderId,
    AiProviderProfile, AiProviderProfileId, QwenClient,
};
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use serde::{Deserialize, Serialize};

use crate::{ApiError, ApiState};

/// Build local, process-memory AI configuration routes.
pub(crate) fn router() -> Router<ApiState> {
    Router::new()
        .route(
            "/ai/session-provider",
            post(configure_session_provider).delete(clear_session_provider),
        )
        .route("/ai/session-provider/test", post(test_session_provider))
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SessionAiProviderKind {
    QwenCloud,
    Qwen,
    Gpt,
    Claude,
    Deepseek,
}

impl SessionAiProviderKind {
    const fn settings(self) -> (&'static str, &'static str, &'static str, AiApiProtocol, u64) {
        match self {
            Self::QwenCloud => (
                "qwen-cloud",
                "QwenCloud（本次运行）",
                "https://maas.qwencloudapi.com/compatible-mode/v1",
                AiApiProtocol::OpenAiChatCompletions,
                90,
            ),
            Self::Qwen => (
                "qwen",
                "阿里云百炼（本次运行）",
                "https://dashscope.aliyuncs.com/compatible-mode",
                AiApiProtocol::OpenAiChatCompletions,
                30,
            ),
            Self::Gpt => (
                "gpt",
                "GPT（本次运行）",
                "https://api.openai.com",
                AiApiProtocol::OpenAiResponses,
                30,
            ),
            Self::Claude => (
                "claude",
                "Claude（本次运行）",
                "https://api.anthropic.com",
                AiApiProtocol::AnthropicMessages,
                30,
            ),
            Self::Deepseek => (
                "deepseek",
                "DeepSeek（本次运行）",
                "https://api.deepseek.com",
                AiApiProtocol::OpenAiChatCompletions,
                30,
            ),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigureSessionProviderRequest {
    provider: SessionAiProviderKind,
    model: String,
    api_key: String,
}

#[derive(Debug, Serialize)]
struct ConfigureSessionProviderResponse {
    provider: AiProviderProfile,
    storage: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum SessionAiProbeStatus {
    Available,
    AuthenticationFailed,
    AccessDenied,
    ModelUnavailable,
    RateLimited,
    RequestRejected,
    NetworkUnavailable,
    ProviderUnavailable,
    ResponseInvalid,
}

#[derive(Debug, Serialize)]
struct SessionAiProbeResponse {
    provider: AiProviderProfile,
    status: SessionAiProbeStatus,
}

/// Register one frontend-entered credential in server process memory without probing it.
async fn configure_session_provider(
    State(state): State<ApiState>,
    input: Result<Json<ConfigureSessionProviderRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<ConfigureSessionProviderResponse>, ApiError> {
    let Json(input) = input.map_err(|_| ApiError::BadRequest)?;
    let api_key = input.api_key.trim();
    if api_key.is_empty()
        || api_key.len() > 512
        || api_key.chars().any(char::is_control)
        || input.model.trim().is_empty()
    {
        return Err(ApiError::BadRequest);
    }
    let (provider_slug, display_name, base_url, protocol, timeout_seconds) =
        input.provider.settings();
    let profile = AiProviderProfile::new(
        AiProviderProfileId::new(format!("session-{provider_slug}"))
            .map_err(|_| ApiError::BadRequest)?,
        AiProviderId::new(provider_slug).map_err(|_| ApiError::BadRequest)?,
        display_name.to_owned(),
        input.model.trim().to_owned(),
        AiProviderCapabilities::market_evidence_and_restricted_policy_drafts(),
    )
    .map_err(|_| ApiError::BadRequest)?;
    let client = QwenClient::with_protocol(
        AiConfig {
            base_url: base_url.to_owned(),
            api_key: api_key.to_owned(),
            model: profile.model().to_owned(),
            timeout: Duration::from_secs(timeout_seconds),
            max_tokens: 2_048,
            temperature: 0.1,
        },
        profile,
        protocol,
    );
    let provider = state.set_session_ai_provider(Arc::new(client) as Arc<dyn AiProvider>)?;
    Ok(Json(ConfigureSessionProviderResponse {
        provider,
        storage: "process_memory",
    }))
}

/// Clear the frontend-entered credential from server process memory.
async fn clear_session_provider(State(state): State<ApiState>) -> Result<StatusCode, ApiError> {
    state.clear_session_ai_provider()?;
    Ok(StatusCode::NO_CONTENT)
}

/// Execute one user-triggered, minimal provider request and return only a safe failure category.
async fn test_session_provider(
    State(state): State<ApiState>,
) -> Result<Json<SessionAiProbeResponse>, ApiError> {
    let (provider, result) = state.probe_session_ai_provider().await?;
    let status = result.map_or_else(
        |error| probe_status(&error),
        |()| SessionAiProbeStatus::Available,
    );
    Ok(Json(SessionAiProbeResponse { provider, status }))
}

fn probe_status(error: &AiClientError) -> SessionAiProbeStatus {
    match error {
        AiClientError::HttpStatus { status: 401 } => SessionAiProbeStatus::AuthenticationFailed,
        AiClientError::HttpStatus { status: 403 } => SessionAiProbeStatus::AccessDenied,
        AiClientError::HttpStatus { status: 404 } => SessionAiProbeStatus::ModelUnavailable,
        AiClientError::HttpStatus { status: 429 } => SessionAiProbeStatus::RateLimited,
        AiClientError::HttpStatus { status: 400 } => SessionAiProbeStatus::RequestRejected,
        AiClientError::Timeout { .. } | AiClientError::Transport(_) => {
            SessionAiProbeStatus::NetworkUnavailable
        }
        AiClientError::HttpStatus { .. } | AiClientError::UnsupportedCapability => {
            SessionAiProbeStatus::ProviderUnavailable
        }
        AiClientError::InvalidJson(_)
        | AiClientError::UnexpectedStructure
        | AiClientError::ParseFailure
        | AiClientError::EmptyResponse => SessionAiProbeStatus::ResponseInvalid,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qwen_cloud_uses_its_pay_as_you_go_openai_compatible_endpoint() {
        let (slug, display_name, base_url, protocol, timeout_seconds) =
            SessionAiProviderKind::QwenCloud.settings();

        assert_eq!(slug, "qwen-cloud");
        assert_eq!(display_name, "QwenCloud（本次运行）");
        assert_eq!(base_url, "https://maas.qwencloudapi.com/compatible-mode/v1");
        assert_eq!(protocol, AiApiProtocol::OpenAiChatCompletions);
        assert_eq!(timeout_seconds, 90);
    }

    #[test]
    fn dashscope_remains_a_distinct_qwen_provider() {
        let (slug, display_name, base_url, protocol, timeout_seconds) =
            SessionAiProviderKind::Qwen.settings();

        assert_eq!(slug, "qwen");
        assert_eq!(display_name, "阿里云百炼（本次运行）");
        assert_eq!(base_url, "https://dashscope.aliyuncs.com/compatible-mode");
        assert_eq!(protocol, AiApiProtocol::OpenAiChatCompletions);
        assert_eq!(timeout_seconds, 30);
    }

    #[test]
    fn provider_probe_errors_map_to_safe_actionable_categories() {
        assert_eq!(
            probe_status(&AiClientError::HttpStatus { status: 401 }),
            SessionAiProbeStatus::AuthenticationFailed
        );
        assert_eq!(
            probe_status(&AiClientError::HttpStatus { status: 403 }),
            SessionAiProbeStatus::AccessDenied
        );
        assert_eq!(
            probe_status(&AiClientError::HttpStatus { status: 404 }),
            SessionAiProbeStatus::ModelUnavailable
        );
        assert_eq!(
            probe_status(&AiClientError::HttpStatus { status: 429 }),
            SessionAiProbeStatus::RateLimited
        );
        assert_eq!(
            probe_status(&AiClientError::HttpStatus { status: 400 }),
            SessionAiProbeStatus::RequestRejected
        );
        assert_eq!(
            probe_status(&AiClientError::EmptyResponse),
            SessionAiProbeStatus::ResponseInvalid
        );
    }
}
