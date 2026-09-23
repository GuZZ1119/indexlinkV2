//! Process-memory read-only OpenD configuration for the local Advanced Lab.

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use market_data::{
    HistoricalPriceProvider, MarketSignalProvider, OpenDHistoricalPriceProvider,
    OpenDMarketSignalProvider,
};
use serde::{Deserialize, Serialize};

use crate::{ApiError, ApiState};

/// Build process-memory OpenD configuration routes.
pub(crate) fn router() -> Router<ApiState> {
    Router::new().route(
        "/market-data/session-opend",
        post(configure_session_opend).delete(clear_session_opend),
    )
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigureSessionOpenDRequest {
    host: String,
    port: u16,
}

#[derive(Debug, Serialize)]
struct ConfigureSessionOpenDResponse {
    provider: &'static str,
    host: String,
    port: u16,
    storage: &'static str,
    access: &'static str,
}

/// Register loopback-only read-only OpenD adapters without probing or enabling broker access.
async fn configure_session_opend(
    State(state): State<ApiState>,
    input: Result<Json<ConfigureSessionOpenDRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<ConfigureSessionOpenDResponse>, ApiError> {
    let Json(input) = input.map_err(|_| ApiError::BadRequest)?;
    let host = input.host.trim();
    let market_data =
        OpenDMarketSignalProvider::new(host, input.port).map_err(|_| ApiError::BadRequest)?;
    let historical_prices =
        OpenDHistoricalPriceProvider::new(host, input.port).map_err(|_| ApiError::BadRequest)?;
    state.set_session_market_data(
        Arc::new(market_data) as Arc<dyn MarketSignalProvider>,
        Arc::new(historical_prices) as Arc<dyn HistoricalPriceProvider>,
    )?;
    Ok(Json(ConfigureSessionOpenDResponse {
        provider: "opend",
        host: host.to_owned(),
        port: input.port,
        storage: "process_memory",
        access: "read_only_market_data",
    }))
}

/// Clear the process-memory read-only OpenD adapters without touching startup configuration.
async fn clear_session_opend(State(state): State<ApiState>) -> Result<StatusCode, ApiError> {
    state.clear_session_market_data()?;
    Ok(StatusCode::NO_CONTENT)
}
