//! Read-only historical daily-close adapter for a locally running Futu/Moomoo OpenD.

use std::{collections::HashSet, net::IpAddr, time::Duration};

use async_trait::async_trait;
use chrono::Utc;
use serde_json::{json, Value};
use tokio::{net::TcpStream, time::timeout};

use crate::{
    opend_request, Adjustment, DatasetSource, HistoricalPriceBar, HistoricalPriceDataset,
    HistoricalPriceProvider, HistoricalPriceRequest, Market, MarketDataError, OPEND_HISTORY_KLINE,
    OPEND_INIT_CONNECT,
};

const OPEND_TIMEOUT: Duration = Duration::from_secs(10);
const OPEND_DAY_KLINE: i64 = 2;
const MAX_PAGES: usize = 100;

/// Generic OpenD daily-history adapter for US, Hong Kong, Shanghai and Shenzhen securities.
#[derive(Debug, Clone)]
pub struct OpenDHistoricalPriceProvider {
    host: String,
    port: u16,
}

impl OpenDHistoricalPriceProvider {
    /// Create a read-only adapter; raw TCP is restricted to a literal loopback address.
    pub fn new(host: impl Into<String>, port: u16) -> Result<Self, MarketDataError> {
        let host = host.into();
        if !host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
            || port == 0
        {
            return Err(MarketDataError::OpenDUnavailable);
        }
        Ok(Self { host, port })
    }
}

#[async_trait]
impl HistoricalPriceProvider for OpenDHistoricalPriceProvider {
    fn provider_id(&self) -> &'static str {
        "opend"
    }

    fn preferred_adjustment(&self, _market: Market) -> Result<Adjustment, MarketDataError> {
        Ok(Adjustment::Forward)
    }

    async fn fetch_history(
        &self,
        request: &HistoricalPriceRequest,
    ) -> Result<HistoricalPriceDataset, MarketDataError> {
        let market = match request.instrument().market() {
            Market::HongKong => 1,
            Market::Us => 11,
            Market::ChinaShanghai => 21,
            Market::ChinaShenzhen => 22,
        };
        let rehab_type = match request.adjustment() {
            Adjustment::Raw => 0,
            Adjustment::Forward => 1,
            Adjustment::Backward => 2,
            Adjustment::Split | Adjustment::Dividend | Adjustment::SpinOff | Adjustment::All => {
                return Err(MarketDataError::UnsupportedRequest)
            }
        };
        let mut stream = timeout(
            OPEND_TIMEOUT,
            TcpStream::connect(format!("{}:{}", self.host, self.port)),
        )
        .await
        .map_err(|_| MarketDataError::OpenDUnavailable)?
        .map_err(|_| MarketDataError::OpenDUnavailable)?;
        let mut serial = 1_u32;
        let init = opend_request(
            &mut stream,
            OPEND_INIT_CONNECT,
            serial,
            json!({"c2s": {"clientVer": 1, "clientID": "indexlink-history-import", "recvNotify": false, "packetEncAlgo": 0, "pushProtoFmt": 1}}),
        )
        .await?;
        validate_response(&init)?;
        serial += 1;

        let mut bars = Vec::new();
        let mut next_request_key: Option<String> = None;
        let mut seen_tokens = HashSet::new();
        for _ in 0..MAX_PAGES {
            let mut body = json!({"c2s": {
                "rehabType": rehab_type,
                "klType": OPEND_DAY_KLINE,
                "security": {"market": market, "code": request.instrument().symbol()},
                "beginTime": request.start().format("%Y-%m-%d").to_string(),
                "endTime": request.end().format("%Y-%m-%d").to_string(),
                "maxAckKLNum": 1000
            }});
            if let Some(token) = next_request_key.take() {
                body["c2s"]["nextReqKey"] = Value::String(token);
            }
            let response = opend_request(&mut stream, OPEND_HISTORY_KLINE, serial, body).await?;
            serial = serial
                .checked_add(1)
                .ok_or(MarketDataError::InvalidDataset)?;
            validate_response(&response)?;
            let payload = response
                .get("s2c")
                .and_then(Value::as_object)
                .ok_or(MarketDataError::InvalidDataset)?;
            let rows = payload
                .get("klList")
                .and_then(Value::as_array)
                .ok_or(MarketDataError::InvalidDataset)?;
            for row in rows {
                let timestamp = row
                    .get("time")
                    .and_then(Value::as_str)
                    .ok_or(MarketDataError::InvalidDataset)?;
                let date = timestamp.get(..10).ok_or(MarketDataError::InvalidDataset)?;
                let date = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
                    .map_err(|_| MarketDataError::InvalidDataset)?;
                let close = row
                    .get("closePrice")
                    .and_then(Value::as_f64)
                    .ok_or(MarketDataError::InvalidDataset)?;
                bars.push(HistoricalPriceBar::new(date, close)?);
            }
            next_request_key = payload
                .get("nextReqKey")
                .and_then(Value::as_str)
                .filter(|token| !token.is_empty())
                .map(ToOwned::to_owned);
            let Some(token) = &next_request_key else {
                return HistoricalPriceDataset::new(
                    request,
                    DatasetSource::new("opend", "history-kline-v10")?,
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

fn validate_response(response: &Value) -> Result<(), MarketDataError> {
    match response.get("retType").and_then(Value::as_i64) {
        Some(0) | None => Ok(()),
        Some(_) => Err(MarketDataError::OpenDUnavailable),
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::Instrument;

    #[test]
    fn accepts_all_supported_qualified_opend_markets() {
        let provider = OpenDHistoricalPriceProvider::new("127.0.0.1", 11111).unwrap();
        for symbol in ["US.AAPL", "HK.00700", "SH.600519", "SZ.000001"] {
            let instrument = Instrument::parse(symbol).unwrap();
            assert_eq!(
                provider.preferred_adjustment(instrument.market()).unwrap(),
                Adjustment::Forward
            );
            HistoricalPriceRequest::new(
                instrument,
                NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2024, 2, 1).unwrap(),
                Adjustment::Forward,
            )
            .unwrap();
        }
        assert!(OpenDHistoricalPriceProvider::new("opend.local", 11111).is_err());
    }

    #[test]
    fn rejects_provider_error_responses_without_exposing_message() {
        assert_eq!(
            validate_response(&json!({"retType": -1, "retMsg": "account detail"})),
            Err(MarketDataError::OpenDUnavailable)
        );
    }
}
