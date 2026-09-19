pub(crate) mod decision_preview;
mod decision_records;
mod health;
mod investment_plans;
mod manual_executions;
mod market_data;
mod market_sentiment;
mod paper_performance;
mod paper_portfolio;
mod ready;
mod runtime_status;
mod signals;
mod strategies;
mod strategy_catalog;

use axum::{routing::get, Router};

use crate::ApiState;

pub(crate) fn router() -> Router<ApiState> {
    Router::new()
        .route("/health", get(health::health))
        .route("/ready", get(ready::ready))
        .route("/runtime-status", get(runtime_status::runtime_status))
        .merge(decision_preview::router())
        .merge(decision_records::router())
        .merge(investment_plans::router())
        .merge(manual_executions::router())
        .merge(market_sentiment::router())
        .merge(market_data::router())
        .merge(paper_portfolio::router())
        .merge(paper_performance::router())
        .merge(signals::router())
        .merge(strategy_catalog::router())
        .merge(strategies::router())
}
