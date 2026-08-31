//! Market-sentiment preview HTTP route.

use ai_client::{AiEvidence, AiProviderProfile};
use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{ApiError, ApiState};

/// Build market-sentiment preview routes.
pub(crate) fn router() -> Router<ApiState> {
    Router::new()
        .route("/ai/providers", get(list_ai_providers))
        .route("/market-sentiment/preview", post(preview_market_sentiment))
}

/// Query parameters accepted by the direct, non-trading AI evidence preview.
#[derive(Debug, Deserialize)]
struct MarketSentimentPreviewQuery {
    /// Optional server-deployed profile ID; an unknown ID is rejected safely.
    profile_id: Option<String>,
}

/// Credential-free list of profiles that this server has actually deployed.
#[derive(Debug, Serialize)]
struct AiProviderListResponse {
    /// Profiles selectable by direct AI evidence preview.
    providers: Vec<AiProviderProfile>,
}

/// Return deployed AI profiles without probing them or exposing credentials.
async fn list_ai_providers(State(state): State<ApiState>) -> Json<AiProviderListResponse> {
    Json(AiProviderListResponse {
        providers: state.ai_provider_profiles(),
    })
}

/// Fetch current market news and derive generic AI evidence through a deployed profile.
async fn preview_market_sentiment(
    State(state): State<ApiState>,
    Query(query): Query<MarketSentimentPreviewQuery>,
) -> Result<Json<MarketSentimentResponse>, ApiError> {
    let evidence = state
        .ai_evidence_for_profile(query.profile_id.as_deref())
        .await?;
    Ok(Json(MarketSentimentResponse::from(&evidence)))
}

/// API response for one market-sentiment preview.
#[derive(Debug, Serialize)]
pub(crate) struct MarketSentimentResponse {
    /// Credential-free profile that produced this evidence.
    provider: AiProviderProfile,
    /// Bounded AI evidence score in `[-1.0, 1.0]`.
    score: f64,
    /// Stable presentation label derived from the score sign.
    label: MarketSentimentLabel,
    /// Concise model explanation grounded in the supplied headlines.
    rationale: String,
    /// Model-supplied uncertainty or risk cautions.
    warnings: Vec<String>,
    /// RSS headlines actually supplied to the model.
    headlines: Vec<MarketSentimentHeadlineResponse>,
}

/// Presentation label for a market-sentiment score.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MarketSentimentLabel {
    /// Positive Qwen sentiment.
    Positive,
    /// Neutral Qwen sentiment.
    Neutral,
    /// Negative Qwen sentiment.
    Negative,
}

/// One source headline included in the market-sentiment response.
#[derive(Debug, Serialize)]
pub(crate) struct MarketSentimentHeadlineResponse {
    /// Original RSS title.
    title: String,
    /// Original RSS HTTP(S) URL when available.
    url: String,
    /// UTC RFC3339 publication timestamp.
    published_at: String,
}

impl From<&AiEvidence> for MarketSentimentResponse {
    fn from(report: &AiEvidence) -> Self {
        let score = report.analysis.sentiment().value();
        let label = if score > 0.0 {
            MarketSentimentLabel::Positive
        } else if score < 0.0 {
            MarketSentimentLabel::Negative
        } else {
            MarketSentimentLabel::Neutral
        };

        Self {
            provider: report.provider.clone(),
            score,
            label,
            rationale: report.analysis.rationale().to_owned(),
            warnings: report.analysis.warnings().to_vec(),
            headlines: report
                .headlines
                .iter()
                .map(|headline| MarketSentimentHeadlineResponse {
                    title: headline.title.clone(),
                    url: headline.url.clone(),
                    published_at: headline.published_at.to_rfc3339(),
                })
                .collect(),
        }
    }
}
