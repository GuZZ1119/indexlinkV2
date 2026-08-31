//! Display-safe server runtime status route.

use ai_client::AiProviderProfile;
use axum::{extract::State, Json};
use serde::Serialize;

use crate::ApiState;

/// Runtime status assembled from local composition and a non-invasive SQLite readiness check.
#[derive(Debug, Serialize)]
pub(crate) struct RuntimeStatusResponse {
    /// Process-level health; this endpoint itself was served by the API process.
    service: &'static str,
    /// SQLite readiness observed while serving this response.
    database: &'static str,
    /// Whether the server composed a market-data adapter from local OpenD settings.
    market_data: &'static str,
    /// Whether the server composed Qwen/news dependencies from local settings.
    qwen: &'static str,
    /// Credential-free AI profiles available to direct evidence preview.
    ai_provider_profiles: Vec<AiProviderProfile>,
    /// Whether the server composed the real paper broker instead of the in-memory fallback.
    paper_broker: &'static str,
    /// The scheduler's safe counters and timestamps.
    scheduler: crate::SchedulerStatus,
}

/// Return capability and scheduler status without invoking paid AI calls or paper trading actions.
pub(crate) async fn runtime_status(State(state): State<ApiState>) -> Json<RuntimeStatusResponse> {
    let capabilities = state.runtime_capabilities();
    let database = if state.check_readiness().await.is_ok() {
        "ready"
    } else {
        "unavailable"
    };
    Json(RuntimeStatusResponse {
        service: "running",
        database,
        market_data: if capabilities.market_data_configured {
            "configured"
        } else {
            "not_configured"
        },
        qwen: if capabilities.qwen_configured {
            "configured"
        } else {
            "not_configured"
        },
        ai_provider_profiles: capabilities.ai_provider_profiles,
        paper_broker: if capabilities.paper_broker_configured {
            "configured"
        } else {
            "not_configured"
        },
        scheduler: capabilities.scheduler,
    })
}
