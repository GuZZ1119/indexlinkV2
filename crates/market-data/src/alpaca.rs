//! Alpaca Market Data v2 historical stock-bars adapter.

use std::{collections::HashSet, fmt, time::Duration};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::{Client, StatusCode, Url};
use serde::Deserialize;

use crate::{
    Adjustment, DatasetSource, HistoricalPriceBar, HistoricalPriceDataset, HistoricalPriceProvider,
    HistoricalPriceRequest, Market, MarketDataError,
};

const ALPACA_BASE_URL: &str = "https://data.alpaca.markets";
const MAX_PAGES: usize = 1_000;

/// Alpaca US equities data feed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlpacaFeed {
    /// Consolidated Securities Information Processor feed.
    Sip,
    /// Investors Exchange feed, commonly available on the free plan.
    Iex,
    /// Blue Ocean ATS overnight feed.
    Boats,
    /// OTC feed.
    Otc,
}

impl AlpacaFeed {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Sip => "sip",
            Self::Iex => "iex",
            Self::Boats => "boats",
            Self::Otc => "otc",
        }
    }
}

/// Credentialed read-only Alpaca historical bars adapter.
#[derive(Clone)]
pub struct AlpacaHistoricalPriceProvider {
    api_key_id: String,
    secret_key: String,
    feed: AlpacaFeed,
    base_url: Url,
    client: Client,
}

impl fmt::Debug for AlpacaHistoricalPriceProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AlpacaHistoricalPriceProvider")
            .field("api_key_id", &"[REDACTED]")
            .field("secret_key", &"[REDACTED]")
            .field("feed", &self.feed)
            .field("base_url", &self.base_url)
            .finish_non_exhaustive()
    }
}

impl AlpacaHistoricalPriceProvider {
    /// Build a provider from user-supplied Alpaca market-data credentials.
    pub fn new(
        api_key_id: impl Into<String>,
        secret_key: impl Into<String>,
        feed: AlpacaFeed,
    ) -> Result<Self, MarketDataError> {
        Self::with_base_url(api_key_id, secret_key, feed, ALPACA_BASE_URL)
    }

    fn with_base_url(
        api_key_id: impl Into<String>,
        secret_key: impl Into<String>,
        feed: AlpacaFeed,
        base_url: &str,
    ) -> Result<Self, MarketDataError> {
        let api_key_id = api_key_id.into();
        let secret_key = secret_key.into();
        if !valid_secret(&api_key_id) || !valid_secret(&secret_key) {
            return Err(MarketDataError::AuthenticationFailed);
        }
        let base_url = Url::parse(base_url).map_err(|_| MarketDataError::ProviderUnavailable)?;
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("IndexLink/0.1 historical-market-data-import")
            .build()
            .map_err(|_| MarketDataError::ProviderUnavailable)?;
        Ok(Self {
            api_key_id,
            secret_key,
            feed,
            base_url,
            client,
        })
    }
}

fn valid_secret(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && !value.chars().any(char::is_control)
        && value.trim() == value
}

#[async_trait]
impl HistoricalPriceProvider for AlpacaHistoricalPriceProvider {
    fn provider_id(&self) -> &'static str {
        "alpaca"
    }

    async fn fetch_history(
        &self,
        request: &HistoricalPriceRequest,
    ) -> Result<HistoricalPriceDataset, MarketDataError> {
        if request.instrument().market() != Market::Us {
            return Err(MarketDataError::UnsupportedRequest);
        }
        let adjustment = match request.adjustment() {
            Adjustment::Raw => "raw",
            Adjustment::Split => "split",
            Adjustment::Dividend => "dividend",
            Adjustment::SpinOff => "spin-off",
            Adjustment::All => "all",
            Adjustment::Forward | Adjustment::Backward => {
                return Err(MarketDataError::UnsupportedRequest)
            }
        };
        let endpoint = self
            .base_url
            .join(&format!(
                "/v2/stocks/{}/bars",
                request.instrument().symbol()
            ))
            .map_err(|_| MarketDataError::ProviderUnavailable)?;
        let mut bars = Vec::new();
        let mut page_token: Option<String> = None;
        let mut seen_tokens = HashSet::new();
        for _ in 0..MAX_PAGES {
            let mut query = vec![
                ("timeframe", "1Day".to_owned()),
                ("start", request.start().to_string()),
                ("end", request.end().to_string()),
                ("limit", "10000".to_owned()),
                ("adjustment", adjustment.to_owned()),
                ("feed", self.feed.as_str().to_owned()),
                ("currency", "USD".to_owned()),
                ("sort", "asc".to_owned()),
            ];
            if let Some(token) = &page_token {
                query.push(("page_token", token.clone()));
            }
            let response = self
                .client
                .get(endpoint.clone())
                .header("APCA-API-KEY-ID", &self.api_key_id)
                .header("APCA-API-SECRET-KEY", &self.secret_key)
                .query(&query)
                .send()
                .await
                .map_err(|_| MarketDataError::ProviderUnavailable)?;
            match response.status() {
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                    return Err(MarketDataError::AuthenticationFailed)
                }
                StatusCode::TOO_MANY_REQUESTS => return Err(MarketDataError::RateLimited),
                status if !status.is_success() => return Err(MarketDataError::ProviderUnavailable),
                _ => {}
            }
            let page: AlpacaBarsResponse = response
                .json()
                .await
                .map_err(|_| MarketDataError::InvalidDataset)?;
            bars.extend(
                page.bars
                    .into_iter()
                    .map(|bar| {
                        let timestamp = DateTime::parse_from_rfc3339(&bar.timestamp)
                            .map_err(|_| MarketDataError::InvalidDataset)?;
                        HistoricalPriceBar::new(timestamp.date_naive(), bar.close)
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            );
            page_token = page.next_page_token.filter(|token| !token.is_empty());
            let Some(token) = &page_token else {
                return HistoricalPriceDataset::new(
                    request,
                    DatasetSource::new("alpaca", "stock-bars-v2")?,
                    Utc::now(),
                    bars,
                );
            };
            if !seen_tokens.insert(token.clone()) {
                return Err(MarketDataError::InvalidDataset);
            }
        }
        Err(MarketDataError::InvalidDataset)
    }
}

#[derive(Debug, Deserialize)]
struct AlpacaBarsResponse {
    #[serde(default)]
    bars: Vec<AlpacaBar>,
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AlpacaBar {
    #[serde(rename = "t")]
    timestamp: String,
    #[serde(rename = "c")]
    close: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_are_never_exposed_by_debug() {
        let provider =
            AlpacaHistoricalPriceProvider::new("public-id", "secret-value", AlpacaFeed::Iex)
                .unwrap();
        let output = format!("{provider:?}");
        assert!(!output.contains("public-id"));
        assert!(!output.contains("secret-value"));
        assert!(AlpacaHistoricalPriceProvider::new("bad\nkey", "secret", AlpacaFeed::Iex).is_err());
    }

    #[test]
    fn response_contract_uses_daily_timestamp_and_close_fields() {
        let page: AlpacaBarsResponse = serde_json::from_str(
            r#"{"bars":[{"t":"2024-01-02T05:00:00Z","c":185.64}],"next_page_token":"next"}"#,
        )
        .unwrap();
        assert_eq!(page.bars.len(), 1);
        assert_eq!(page.bars[0].close, 185.64);
        assert_eq!(page.next_page_token.as_deref(), Some("next"));
    }
}
