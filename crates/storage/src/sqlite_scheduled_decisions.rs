//! SQLite idempotency ledger for periodic automatic decision runs.

use sqlx::SqlitePool;
use uuid::Uuid;

const CLAIM_RUN_SQL: &str =
    "INSERT OR IGNORE INTO scheduled_decision_runs (plan_id, scheduled_for) VALUES (?1, ?2)";

/// SQLite repository used to claim one automatic decision run per plan-local scheduled day.
#[derive(Clone, Debug)]
pub struct SqliteScheduledDecisionRepository {
    pool: SqlitePool,
}

impl SqliteScheduledDecisionRepository {
    /// Build a scheduler idempotency repository from an existing SQLite pool.
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Atomically claim a scheduled decision run.
    ///
    /// Returns `true` only for the first claimant of the `(plan_id, scheduled_for)` pair.
    /// A caller must use an ISO `YYYY-MM-DD` UTC calendar date.
    pub async fn claim(&self, plan_id: Uuid, scheduled_for: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(CLAIM_RUN_SQL)
            .bind(plan_id.to_string())
            .bind(scheduled_for)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() == 1)
    }

    /// Release a just-claimed run when its decision record could not be persisted.
    ///
    /// Only callers that own the claim should invoke this; a persisted record must never be
    /// released because the next scheduler tick would duplicate its audit trail.
    pub async fn release(&self, plan_id: Uuid, scheduled_for: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "DELETE FROM scheduled_decision_runs WHERE plan_id = ?1 AND scheduled_for = ?2",
        )
        .bind(plan_id.to_string())
        .bind(scheduled_for)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::SqliteStorage;

    /// Verify a plan/day key is claimed exactly once across repository handles.
    #[tokio::test]
    async fn claims_each_plan_and_scheduled_day_only_once() {
        let storage =
            SqliteStorage::connect_with_options("sqlite::memory:", 1, Duration::from_secs(1))
                .await
                .expect("in-memory SQLite should connect");
        storage.migrate().await.expect("migrations should apply");
        let first = SqliteScheduledDecisionRepository::new(storage.pool().clone());
        let second = SqliteScheduledDecisionRepository::new(storage.pool().clone());
        let plan_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO investment_plans (id, name, symbol, base_contribution, currency, schedule_day, max_single_execution) VALUES (?1, 'Scheduler test', 'VOO', '000000000100.00000000', 'USD', 15, '000000000100.00000000')",
        )
        .bind(plan_id.to_string())
        .execute(storage.pool())
        .await
        .expect("test plan should be inserted before its scheduler claim");

        assert!(first.claim(plan_id, "2026-07-19").await.unwrap());
        assert!(!second.claim(plan_id, "2026-07-19").await.unwrap());
        assert!(second.claim(plan_id, "2026-07-20").await.unwrap());
    }
}
