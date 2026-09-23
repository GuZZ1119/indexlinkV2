//! SQLite adapter for exact, checksummed historical daily-price snapshots.

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use market_data::{
    Adjustment, DatasetSource, HistoricalPriceBar, HistoricalPriceDataset, HistoricalPriceRequest,
    Instrument, MarketDataError, PriceHistoryStore,
};
use sqlx::{Row, SqlitePool};

/// Local canonical history store used behind explicit remote import adapters.
#[derive(Clone, Debug)]
pub struct SqlitePriceHistoryStore {
    pool: SqlitePool,
}

impl SqlitePriceHistoryStore {
    /// Build the adapter from an existing migrated SQLite pool.
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PriceHistoryStore for SqlitePriceHistoryStore {
    async fn load_history(
        &self,
        provider: &str,
        request: &HistoricalPriceRequest,
    ) -> Result<Option<HistoricalPriceDataset>, MarketDataError> {
        let row = sqlx::query(
            "SELECT id, instrument_type, currency, timezone, fetched_at, dataset_version, checksum \
             FROM market_price_datasets \
             WHERE provider = ?1 AND market = ?2 AND symbol = ?3 AND adjustment = ?4 \
             AND requested_start = ?5 AND requested_end = ?6",
        )
        .bind(provider)
        .bind(request.instrument().market().as_str())
        .bind(request.instrument().symbol())
        .bind(request.adjustment().as_str())
        .bind(request.start().to_string())
        .bind(request.end().to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| MarketDataError::StoreUnavailable)?;
        let Some(row) = row else {
            return Ok(None);
        };
        let dataset_id: i64 = row
            .try_get("id")
            .map_err(|_| MarketDataError::StoreUnavailable)?;
        let bars = sqlx::query(
            "SELECT trading_date, close FROM market_price_bars \
             WHERE dataset_id = ?1 ORDER BY trading_date ASC",
        )
        .bind(dataset_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| MarketDataError::StoreUnavailable)?
        .into_iter()
        .map(|bar| {
            let date: String = bar
                .try_get("trading_date")
                .map_err(|_| MarketDataError::InvalidDataset)?;
            let close: f64 = bar
                .try_get("close")
                .map_err(|_| MarketDataError::InvalidDataset)?;
            HistoricalPriceBar::new(
                NaiveDate::parse_from_str(&date, "%Y-%m-%d")
                    .map_err(|_| MarketDataError::InvalidDataset)?,
                close,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
        let instrument_type: String = row
            .try_get("instrument_type")
            .map_err(|_| MarketDataError::InvalidDataset)?;
        let currency: String = row
            .try_get("currency")
            .map_err(|_| MarketDataError::InvalidDataset)?;
        let timezone: String = row
            .try_get("timezone")
            .map_err(|_| MarketDataError::InvalidDataset)?;
        let fetched_at: String = row
            .try_get("fetched_at")
            .map_err(|_| MarketDataError::InvalidDataset)?;
        let dataset_version: String = row
            .try_get("dataset_version")
            .map_err(|_| MarketDataError::InvalidDataset)?;
        let checksum: String = row
            .try_get("checksum")
            .map_err(|_| MarketDataError::InvalidDataset)?;
        HistoricalPriceDataset::restore(
            Instrument::restore(
                request.instrument().market().as_str(),
                request.instrument().symbol(),
                &instrument_type,
                &currency,
                &timezone,
            )?,
            Adjustment::parse(request.adjustment().as_str())?,
            DatasetSource::new(provider, &dataset_version)?,
            DateTime::parse_from_rfc3339(&fetched_at)
                .map_err(|_| MarketDataError::InvalidDataset)?
                .with_timezone(&Utc),
            request.start(),
            request.end(),
            &checksum,
            bars,
        )
        .map(Some)
    }

    async fn save_history(&self, dataset: &HistoricalPriceDataset) -> Result<(), MarketDataError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| MarketDataError::StoreUnavailable)?;
        let row = sqlx::query(
            "INSERT INTO market_price_datasets \
             (provider, market, symbol, instrument_type, currency, timezone, adjustment, \
              requested_start, requested_end, fetched_at, dataset_version, checksum) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12) \
             ON CONFLICT(provider, market, symbol, adjustment, requested_start, requested_end) \
             DO UPDATE SET instrument_type = excluded.instrument_type, currency = excluded.currency, \
             timezone = excluded.timezone, fetched_at = excluded.fetched_at, \
             dataset_version = excluded.dataset_version, checksum = excluded.checksum \
             RETURNING id",
        )
        .bind(dataset.source().provider())
        .bind(dataset.instrument().market().as_str())
        .bind(dataset.instrument().symbol())
        .bind(dataset.instrument().instrument_type().as_str())
        .bind(dataset.instrument().currency())
        .bind(dataset.instrument().timezone())
        .bind(dataset.adjustment().as_str())
        .bind(dataset.requested_start().to_string())
        .bind(dataset.requested_end().to_string())
        .bind(dataset.fetched_at().to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        .bind(dataset.source().dataset_version())
        .bind(dataset.checksum())
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| MarketDataError::StoreUnavailable)?;
        let dataset_id: i64 = row
            .try_get("id")
            .map_err(|_| MarketDataError::StoreUnavailable)?;
        sqlx::query("DELETE FROM market_price_bars WHERE dataset_id = ?1")
            .bind(dataset_id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| MarketDataError::StoreUnavailable)?;
        for bar in dataset.bars() {
            sqlx::query(
                "INSERT INTO market_price_bars (dataset_id, trading_date, close) VALUES (?1, ?2, ?3)",
            )
            .bind(dataset_id)
            .bind(bar.date().to_string())
            .bind(bar.close())
            .execute(&mut *transaction)
            .await
            .map_err(|_| MarketDataError::StoreUnavailable)?;
        }
        transaction
            .commit()
            .await
            .map_err(|_| MarketDataError::StoreUnavailable)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::TimeZone;
    use market_data::{Adjustment, HistoricalPriceRequest, Market};

    use super::*;
    use crate::SqliteStorage;

    #[tokio::test]
    async fn exact_snapshots_round_trip_without_crossing_provider_or_range() {
        let storage =
            SqliteStorage::connect_with_options("sqlite::memory:", 1, Duration::from_secs(1))
                .await
                .unwrap();
        storage.migrate().await.unwrap();
        let store = SqlitePriceHistoryStore::new(storage.pool().clone());
        let request = HistoricalPriceRequest::new(
            Instrument::new(Market::ChinaShanghai, "600519").unwrap(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2024, 1, 31).unwrap(),
            Adjustment::Forward,
        )
        .unwrap();
        let dataset = HistoricalPriceDataset::new(
            &request,
            DatasetSource::new("opend", "history-kline-v10").unwrap(),
            Utc.with_ymd_and_hms(2024, 2, 1, 0, 0, 0).unwrap(),
            vec![
                HistoricalPriceBar::new(NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(), 1_700.5)
                    .unwrap(),
                HistoricalPriceBar::new(NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(), 1_690.0)
                    .unwrap(),
            ],
        )
        .unwrap();
        store.save_history(&dataset).await.unwrap();

        assert_eq!(
            store.load_history("opend", &request).await.unwrap(),
            Some(dataset)
        );
        assert_eq!(store.load_history("alpaca", &request).await.unwrap(), None);
        let other_range = HistoricalPriceRequest::new(
            request.instrument().clone(),
            request.start(),
            NaiveDate::from_ymd_opt(2024, 2, 1).unwrap(),
            request.adjustment(),
        )
        .unwrap();
        assert_eq!(
            store.load_history("opend", &other_range).await.unwrap(),
            None
        );
    }
}
