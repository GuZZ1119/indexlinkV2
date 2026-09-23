#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! IndexLink HTTP API 基础设施。

mod error;
mod official_strategies;
mod routes;
mod state;

use axum::{
    http::{header::CONTENT_TYPE, HeaderValue, Method},
    Router,
};
use tower_http::{cors::CorsLayer, limit::RequestBodyLimitLayer, trace::TraceLayer};

pub use error::{ApiError, ErrorBody, ErrorEnvelope};
pub use routes::decision_preview::ScheduledDecisionRunSummary;
pub use state::{ApiState, ReadinessCheck, ReadinessError, SchedulerStatus, SchedulerStatusHandle};

const MAX_REQUEST_BODY_BYTES: usize = 1024 * 1024;

/// 使用同源 CORS 策略构建 API router。
pub fn build_router(state: ApiState) -> Router {
    build_router_with_cors(state, Vec::new())
}

/// 使用指定允许来源构建 API router。
///
/// 空列表表示不添加跨域来源；调用方应在启动阶段完成来源值校验。
pub fn build_router_with_cors(state: ApiState, allowed_origins: Vec<HeaderValue>) -> Router {
    let mut cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::PUT,
            Method::DELETE,
        ])
        .allow_headers([CONTENT_TYPE]);
    if !allowed_origins.is_empty() {
        cors = cors.allow_origin(allowed_origins);
    }

    Router::new()
        .merge(routes::router())
        .with_state(state)
        .layer(RequestBodyLimitLayer::new(MAX_REQUEST_BODY_BYTES))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}

/// Run one safe periodic automatic-decision scheduler tick.
///
/// The tick only creates auditable decisions for active plans whose configured UTC calendar day
/// is due according to its persisted monthly/weekly fixed-day set. It never submits a broker order: paper-order submission remains an explicit operator
/// confirmation step. A local SQLite idempotency ledger prevents duplicate automatic records for
/// the same plan and UTC day.
///
/// # Errors
///
/// Returns [`ApiError::ServiceUnavailable`] only when the plan store itself cannot be read.
/// Per-plan market-data failures are counted in the returned summary so the next tick can retry.
pub async fn run_due_decisions(state: &ApiState) -> Result<ScheduledDecisionRunSummary, ApiError> {
    routes::decision_preview::run_due_decisions(state).await
}
