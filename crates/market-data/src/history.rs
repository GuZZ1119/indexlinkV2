//! Canonical historical daily-close contracts shared by import adapters and local storage.

use std::collections::HashSet;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::MarketDataError;

/// Safety bound for one inclusive historical request (approximately 150 years).
pub const MAX_HISTORY_SPAN_DAYS: i64 = 366 * 150;

/// Supported exchange market for one listed security.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Market {
    /// United States listed securities.
    Us,
    /// Hong Kong listed securities.
    HongKong,
    /// Shanghai listed A shares and ETFs.
    ChinaShanghai,
    /// Shenzhen listed A shares and ETFs.
    ChinaShenzhen,
}

impl Market {
    /// Stable storage/API value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Us => "us",
            Self::HongKong => "hong_kong",
            Self::ChinaShanghai => "china_shanghai",
            Self::ChinaShenzhen => "china_shenzhen",
        }
    }

    /// ISO 4217 trading currency used by this market adapter.
    #[must_use]
    pub const fn currency(self) -> &'static str {
        match self {
            Self::Us => "USD",
            Self::HongKong => "HKD",
            Self::ChinaShanghai | Self::ChinaShenzhen => "CNY",
        }
    }

    /// IANA exchange timezone used to interpret a daily trading date.
    #[must_use]
    pub const fn timezone(self) -> &'static str {
        match self {
            Self::Us => "America/New_York",
            Self::HongKong => "Asia/Hong_Kong",
            Self::ChinaShanghai | Self::ChinaShenzhen => "Asia/Shanghai",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, MarketDataError> {
        match value {
            "us" => Ok(Self::Us),
            "hong_kong" => Ok(Self::HongKong),
            "china_shanghai" => Ok(Self::ChinaShanghai),
            "china_shenzhen" => Ok(Self::ChinaShenzhen),
            _ => Err(MarketDataError::InvalidDataset),
        }
    }
}

/// Instrument classification retained with each cached dataset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentType {
    /// A listed stock or ETF. OpenD/Alpaca bars do not reliably distinguish the two.
    EquityOrEtf,
}

impl InstrumentType {
    /// Stable storage/API value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        "equity_or_etf"
    }
}

/// A validated, market-qualified security identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct Instrument {
    market: Market,
    symbol: String,
    instrument_type: InstrumentType,
    currency: String,
    timezone: String,
}

impl Instrument {
    /// Build a listed stock/ETF identifier and derive canonical market metadata.
    pub fn new(market: Market, symbol: &str) -> Result<Self, MarketDataError> {
        let mut symbol = symbol.trim().to_ascii_uppercase();
        if market == Market::HongKong
            && symbol.len() < 5
            && symbol.bytes().all(|byte| byte.is_ascii_digit())
        {
            symbol = format!("{symbol:0>5}");
        }
        let valid = match market {
            Market::Us => {
                !symbol.is_empty()
                    && symbol.len() <= 15
                    && symbol.bytes().all(|byte| {
                        byte.is_ascii_uppercase()
                            || byte.is_ascii_digit()
                            || byte == b'.'
                            || byte == b'-'
                    })
            }
            Market::HongKong => {
                (1..=6).contains(&symbol.len()) && symbol.bytes().all(|byte| byte.is_ascii_digit())
            }
            Market::ChinaShanghai | Market::ChinaShenzhen => {
                symbol.len() == 6 && symbol.bytes().all(|byte| byte.is_ascii_digit())
            }
        };
        if !valid {
            return Err(MarketDataError::InvalidSymbol);
        }
        Ok(Self {
            market,
            symbol,
            instrument_type: InstrumentType::EquityOrEtf,
            currency: market.currency().to_owned(),
            timezone: market.timezone().to_owned(),
        })
    }

    /// Parse `US.AAPL`, `HK.00700`, `SH.600519`, or `SZ.000001`.
    /// An unqualified symbol remains a backwards-compatible US symbol.
    pub fn parse(value: &str) -> Result<Self, MarketDataError> {
        let normalized = value.trim().to_ascii_uppercase();
        if let Some((prefix, symbol)) = normalized.split_once('.') {
            match prefix {
                "US" => return Self::new(Market::Us, symbol),
                "HK" => return Self::new(Market::HongKong, symbol),
                "SH" => return Self::new(Market::ChinaShanghai, symbol),
                "SZ" => return Self::new(Market::ChinaShenzhen, symbol),
                _ if prefix.len() == 2 => return Err(MarketDataError::InvalidSymbol),
                _ => {}
            }
        }
        Self::new(Market::Us, &normalized)
    }

    /// Exchange market.
    #[must_use]
    pub const fn market(&self) -> Market {
        self.market
    }

    /// Provider-native symbol without the market prefix.
    #[must_use]
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    /// Instrument classification.
    #[must_use]
    pub const fn instrument_type(&self) -> InstrumentType {
        self.instrument_type
    }

    /// ISO 4217 trading currency.
    #[must_use]
    pub fn currency(&self) -> &str {
        &self.currency
    }

    /// IANA exchange timezone.
    #[must_use]
    pub fn timezone(&self) -> &str {
        &self.timezone
    }

    /// Stable qualified form used in audit output.
    #[must_use]
    pub fn qualified_symbol(&self) -> String {
        let prefix = match self.market {
            Market::Us => "US",
            Market::HongKong => "HK",
            Market::ChinaShanghai => "SH",
            Market::ChinaShenzhen => "SZ",
        };
        format!("{prefix}.{}", self.symbol)
    }

    /// Rebuild persisted metadata while reapplying all market invariants.
    pub fn restore(
        market: &str,
        symbol: &str,
        instrument_type: &str,
        currency: &str,
        timezone: &str,
    ) -> Result<Self, MarketDataError> {
        if instrument_type != InstrumentType::EquityOrEtf.as_str() {
            return Err(MarketDataError::InvalidDataset);
        }
        let instrument = Self::new(Market::parse(market)?, symbol)?;
        if instrument.currency != currency || instrument.timezone != timezone {
            return Err(MarketDataError::InvalidDataset);
        }
        Ok(instrument)
    }
}

/// Price adjustment applied by the source before bars enter the local store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Adjustment {
    /// No corporate-action adjustment.
    Raw,
    /// Split adjustment.
    Split,
    /// Cash-dividend adjustment.
    Dividend,
    /// Spin-off adjustment.
    SpinOff,
    /// All adjustments supported by Alpaca.
    All,
    /// Forward adjustment supplied by OpenD (`QFQ`).
    Forward,
    /// Backward adjustment supplied by OpenD (`HFQ`).
    Backward,
}

impl Adjustment {
    /// Stable storage/API value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Split => "split",
            Self::Dividend => "dividend",
            Self::SpinOff => "spin_off",
            Self::All => "all",
            Self::Forward => "forward",
            Self::Backward => "backward",
        }
    }

    /// Parse the stable storage/API value while rejecting unknown adjustments.
    pub fn parse(value: &str) -> Result<Self, MarketDataError> {
        match value {
            "raw" => Ok(Self::Raw),
            "split" => Ok(Self::Split),
            "dividend" => Ok(Self::Dividend),
            "spin_off" => Ok(Self::SpinOff),
            "all" => Ok(Self::All),
            "forward" => Ok(Self::Forward),
            "backward" => Ok(Self::Backward),
            _ => Err(MarketDataError::InvalidDataset),
        }
    }
}

/// One bounded historical daily-close request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoricalPriceRequest {
    instrument: Instrument,
    start: NaiveDate,
    end: NaiveDate,
    adjustment: Adjustment,
}

impl HistoricalPriceRequest {
    /// Create a request whose inclusive end is not before its start and whose span is bounded.
    pub fn new(
        instrument: Instrument,
        start: NaiveDate,
        end: NaiveDate,
        adjustment: Adjustment,
    ) -> Result<Self, MarketDataError> {
        if start > end || end.signed_duration_since(start).num_days() > MAX_HISTORY_SPAN_DAYS {
            return Err(MarketDataError::InvalidRange);
        }
        Ok(Self {
            instrument,
            start,
            end,
            adjustment,
        })
    }

    /// Requested instrument.
    #[must_use]
    pub fn instrument(&self) -> &Instrument {
        &self.instrument
    }

    /// Inclusive start date.
    #[must_use]
    pub const fn start(&self) -> NaiveDate {
        self.start
    }

    /// Inclusive end date.
    #[must_use]
    pub const fn end(&self) -> NaiveDate {
        self.end
    }

    /// Requested adjustment.
    #[must_use]
    pub const fn adjustment(&self) -> Adjustment {
        self.adjustment
    }
}

/// One validated daily closing price.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HistoricalPriceBar {
    date: NaiveDate,
    close: f64,
}

impl HistoricalPriceBar {
    /// Create a positive finite daily closing price.
    pub fn new(date: NaiveDate, close: f64) -> Result<Self, MarketDataError> {
        if !close.is_finite() || close <= 0.0 {
            return Err(MarketDataError::InvalidDataset);
        }
        Ok(Self { date, close })
    }

    /// Exchange-local trading date.
    #[must_use]
    pub const fn date(&self) -> NaiveDate {
        self.date
    }

    /// Adjusted or raw close according to dataset metadata.
    #[must_use]
    pub const fn close(&self) -> f64 {
        self.close
    }
}

/// Provider provenance for a downloaded immutable snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DatasetSource {
    provider: String,
    dataset_version: String,
}

impl DatasetSource {
    /// Create safe bounded provenance labels.
    pub fn new(provider: &str, dataset_version: &str) -> Result<Self, MarketDataError> {
        fn valid(value: &str) -> bool {
            !value.is_empty()
                && value.len() <= 80
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"._-/".contains(&byte))
        }
        if !valid(provider) || !valid(dataset_version) {
            return Err(MarketDataError::InvalidDataset);
        }
        Ok(Self {
            provider: provider.to_owned(),
            dataset_version: dataset_version.to_owned(),
        })
    }

    /// Stable provider identifier.
    #[must_use]
    pub fn provider(&self) -> &str {
        &self.provider
    }

    /// Provider/API dataset contract version.
    #[must_use]
    pub fn dataset_version(&self) -> &str {
        &self.dataset_version
    }
}

/// A complete, independently checksummed response for one request range.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HistoricalPriceDataset {
    instrument: Instrument,
    adjustment: Adjustment,
    source: DatasetSource,
    fetched_at: DateTime<Utc>,
    requested_start: NaiveDate,
    requested_end: NaiveDate,
    checksum: String,
    bars: Vec<HistoricalPriceBar>,
}

impl HistoricalPriceDataset {
    /// Validate ordering, uniqueness, coverage and checksum a provider response.
    pub fn new(
        request: &HistoricalPriceRequest,
        source: DatasetSource,
        fetched_at: DateTime<Utc>,
        mut bars: Vec<HistoricalPriceBar>,
    ) -> Result<Self, MarketDataError> {
        if bars.is_empty() {
            return Err(MarketDataError::InsufficientHistory);
        }
        bars.sort_by_key(HistoricalPriceBar::date);
        let mut seen = HashSet::with_capacity(bars.len());
        if bars
            .iter()
            .any(|bar| bar.date < request.start || bar.date > request.end || !seen.insert(bar.date))
        {
            return Err(MarketDataError::InvalidDataset);
        }
        let checksum = checksum(request, &source, &bars);
        Ok(Self {
            instrument: request.instrument.clone(),
            adjustment: request.adjustment,
            source,
            fetched_at,
            requested_start: request.start,
            requested_end: request.end,
            checksum,
            bars,
        })
    }

    /// Reconstruct a persisted snapshot and reject corrupted metadata or bars.
    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        instrument: Instrument,
        adjustment: Adjustment,
        source: DatasetSource,
        fetched_at: DateTime<Utc>,
        requested_start: NaiveDate,
        requested_end: NaiveDate,
        checksum_value: &str,
        bars: Vec<HistoricalPriceBar>,
    ) -> Result<Self, MarketDataError> {
        let request =
            HistoricalPriceRequest::new(instrument, requested_start, requested_end, adjustment)?;
        let dataset = Self::new(&request, source, fetched_at, bars)?;
        if dataset.checksum != checksum_value {
            return Err(MarketDataError::InvalidDataset);
        }
        Ok(dataset)
    }

    /// Instrument and canonical market metadata.
    #[must_use]
    pub fn instrument(&self) -> &Instrument {
        &self.instrument
    }

    /// Applied adjustment.
    #[must_use]
    pub const fn adjustment(&self) -> Adjustment {
        self.adjustment
    }

    /// Provider provenance.
    #[must_use]
    pub fn source(&self) -> &DatasetSource {
        &self.source
    }

    /// UTC import time.
    #[must_use]
    pub const fn fetched_at(&self) -> DateTime<Utc> {
        self.fetched_at
    }

    /// Requested inclusive start, retained even when it is a non-trading day.
    #[must_use]
    pub const fn requested_start(&self) -> NaiveDate {
        self.requested_start
    }

    /// Requested inclusive end, retained even when it is a non-trading day.
    #[must_use]
    pub const fn requested_end(&self) -> NaiveDate {
        self.requested_end
    }

    /// SHA-256 over canonical metadata and ordered daily closes.
    #[must_use]
    pub fn checksum(&self) -> &str {
        &self.checksum
    }

    /// Strictly ordered real provider bars; missing dates are not synthesized.
    #[must_use]
    pub fn bars(&self) -> &[HistoricalPriceBar] {
        &self.bars
    }
}

fn checksum(
    request: &HistoricalPriceRequest,
    source: &DatasetSource,
    bars: &[HistoricalPriceBar],
) -> String {
    let mut digest = Sha256::new();
    for value in [
        request.instrument.qualified_symbol(),
        request.instrument.instrument_type.as_str().to_owned(),
        request.instrument.currency.clone(),
        request.instrument.timezone.clone(),
        request.adjustment.as_str().to_owned(),
        request.start.to_string(),
        request.end.to_string(),
        source.provider.clone(),
        source.dataset_version.clone(),
    ] {
        digest.update(value.as_bytes());
        digest.update([0]);
    }
    for bar in bars {
        digest.update(bar.date.to_string().as_bytes());
        digest.update([0]);
        digest.update(bar.close.to_bits().to_be_bytes());
    }
    format!("{:x}", digest.finalize())
}

/// Provider-neutral historical daily price port.
#[async_trait]
pub trait HistoricalPriceProvider: Send + Sync {
    /// Stable source identifier used as part of the cache key.
    fn provider_id(&self) -> &'static str;

    /// Return the provider's canonical supported adjustment for one market.
    ///
    /// Callers must not infer protocol-specific adjustment support from a provider name. The
    /// returned value is included in the immutable request and provenance snapshot.
    fn preferred_adjustment(&self, market: Market) -> Result<Adjustment, MarketDataError>;

    /// Explicitly download one complete request range.
    async fn fetch_history(
        &self,
        request: &HistoricalPriceRequest,
    ) -> Result<HistoricalPriceDataset, MarketDataError>;
}

/// Local canonical snapshot store. It never performs network IO.
#[async_trait]
pub trait PriceHistoryStore: Send + Sync {
    /// Load an exact request snapshot for one provider, if already imported.
    async fn load_history(
        &self,
        provider: &str,
        request: &HistoricalPriceRequest,
    ) -> Result<Option<HistoricalPriceDataset>, MarketDataError>;

    /// Atomically replace the exact snapshot identified by its request metadata.
    async fn save_history(&self, dataset: &HistoricalPriceDataset) -> Result<(), MarketDataError>;
}

/// Cache-first decorator. A miss downloads and atomically persists one exact snapshot.
#[derive(Debug, Clone)]
pub struct CachedHistoricalPriceProvider<P, S> {
    provider: P,
    store: S,
}

impl<P, S> CachedHistoricalPriceProvider<P, S> {
    /// Wrap a remote import adapter with a local canonical store.
    #[must_use]
    pub const fn new(provider: P, store: S) -> Self {
        Self { provider, store }
    }
}

#[async_trait]
impl<P, S> HistoricalPriceProvider for CachedHistoricalPriceProvider<P, S>
where
    P: HistoricalPriceProvider,
    S: PriceHistoryStore,
{
    fn provider_id(&self) -> &'static str {
        self.provider.provider_id()
    }

    fn preferred_adjustment(&self, market: Market) -> Result<Adjustment, MarketDataError> {
        self.provider.preferred_adjustment(market)
    }

    async fn fetch_history(
        &self,
        request: &HistoricalPriceRequest,
    ) -> Result<HistoricalPriceDataset, MarketDataError> {
        if let Some(dataset) = self
            .store
            .load_history(self.provider.provider_id(), request)
            .await?
        {
            return Ok(dataset);
        }
        let dataset = self.provider.fetch_history(request).await?;
        if dataset.source.provider != self.provider.provider_id() {
            return Err(MarketDataError::InvalidDataset);
        }
        self.store.save_history(&dataset).await?;
        Ok(dataset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualified_symbols_derive_market_metadata() {
        let hk = Instrument::parse(" hk.00700 ").unwrap();
        assert_eq!(hk.qualified_symbol(), "HK.00700");
        assert_eq!(hk.currency(), "HKD");
        assert_eq!(hk.timezone(), "Asia/Hong_Kong");
        assert_eq!(Instrument::parse("600519").unwrap().market(), Market::Us);
        assert_eq!(Instrument::parse("HK.700").unwrap().symbol(), "00700");
        assert_eq!(Instrument::parse("BRK.B").unwrap().symbol(), "BRK.B");
        assert!(Instrument::parse("SH.AAPL").is_err());
        assert!(Instrument::parse("SZ.1").is_err());
        assert!(Instrument::parse("JP.7974").is_err());
    }

    #[test]
    fn request_rejects_inverted_and_unbounded_ranges() {
        let instrument = Instrument::parse("US.AAPL").unwrap();
        let start = NaiveDate::from_ymd_opt(1800, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        assert_eq!(
            HistoricalPriceRequest::new(instrument.clone(), end, start, Adjustment::All),
            Err(MarketDataError::InvalidRange)
        );
        assert_eq!(
            HistoricalPriceRequest::new(instrument, start, end, Adjustment::All),
            Err(MarketDataError::InvalidRange)
        );
    }

    #[test]
    fn dataset_sorts_bars_and_checksum_detects_tampering() {
        let request = HistoricalPriceRequest::new(
            Instrument::parse("US.AAPL").unwrap(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2024, 1, 31).unwrap(),
            Adjustment::All,
        )
        .unwrap();
        let source = DatasetSource::new("alpaca", "stock-bars-v2").unwrap();
        let dataset = HistoricalPriceDataset::new(
            &request,
            source.clone(),
            Utc::now(),
            vec![
                HistoricalPriceBar::new(NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(), 101.0)
                    .unwrap(),
                HistoricalPriceBar::new(NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(), 100.0)
                    .unwrap(),
            ],
        )
        .unwrap();
        assert!(dataset
            .bars()
            .windows(2)
            .all(|pair| pair[0].date < pair[1].date));
        assert!(HistoricalPriceDataset::restore(
            request.instrument().clone(),
            request.adjustment(),
            source,
            dataset.fetched_at(),
            request.start(),
            request.end(),
            "0bad",
            dataset.bars().to_vec(),
        )
        .is_err());
    }
}
