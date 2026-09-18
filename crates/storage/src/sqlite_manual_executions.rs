//! SQLite adapter for the append-only manual execution journal.

use async_trait::async_trait;
use decision_records::{
    CreateManualExecutionEvent, ManualExecutionEvent, ManualExecutionOutcome,
    ManualExecutionRepository, ManualExecutionRepositoryError, ManualExecutionSource,
};
use rust_decimal::Decimal;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::sqlite::{decode_amount, encode_amount};

const INSERT_EVENT_SQL: &str = concat!(
    "INSERT INTO manual_execution_events ",
    "(id, decision_record_id, plan_id, outcome, actual_amount, currency, note, occurred_at_micros, recorded_at_micros, source) ",
    "SELECT ?1, decision_records.id, decision_records.plan_id, ?2, ?3, decision_records.currency, ?4, ?5, ?6, 'user_reported' ",
    "FROM decision_records WHERE decision_records.id = ?7 ",
    "RETURNING id, decision_record_id, plan_id, outcome, actual_amount, currency, note, ",
    "occurred_at_micros, recorded_at_micros, source"
);
const LIST_EVENTS_BY_DECISION_SQL: &str = concat!(
    "SELECT id, decision_record_id, plan_id, outcome, actual_amount, currency, note, ",
    "occurred_at_micros, recorded_at_micros, source ",
    "FROM manual_execution_events WHERE decision_record_id = ?1 ",
    "ORDER BY recorded_at_micros ASC, id ASC"
);

/// SQLite implementation of the append-only manual execution repository port.
#[derive(Clone, Debug)]
pub struct SqliteManualExecutionRepository {
    pool: SqlitePool,
}

impl SqliteManualExecutionRepository {
    /// Build a repository from an existing SQLite pool.
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ManualExecutionRepository for SqliteManualExecutionRepository {
    async fn append(
        &self,
        input: CreateManualExecutionEvent,
    ) -> Result<ManualExecutionEvent, ManualExecutionRepositoryError> {
        let input = input.normalize()?;
        let actual_amount = input.actual_amount.map(encode_actual_amount).transpose()?;
        let occurred_at_micros = timestamp_to_micros(input.occurred_at)?;
        let recorded_at_micros = timestamp_to_micros(OffsetDateTime::now_utc())?;
        let row = sqlx::query(INSERT_EVENT_SQL)
            .bind(input.id.to_string())
            .bind(outcome_to_str(input.outcome))
            .bind(actual_amount)
            .bind(input.note)
            .bind(occurred_at_micros)
            .bind(recorded_at_micros)
            .bind(input.decision_record_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(ManualExecutionRepositoryError::NotFound)?;

        event_from_row(row)
    }

    async fn list_by_decision(
        &self,
        decision_record_id: Uuid,
    ) -> Result<Vec<ManualExecutionEvent>, ManualExecutionRepositoryError> {
        let rows = sqlx::query(LIST_EVENTS_BY_DECISION_SQL)
            .bind(decision_record_id.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        rows.into_iter().map(event_from_row).collect()
    }
}

fn event_from_row(row: SqliteRow) -> Result<ManualExecutionEvent, ManualExecutionRepositoryError> {
    let source = match column::<String>(&row, "source")?.as_str() {
        "user_reported" => ManualExecutionSource::UserReported,
        _ => return Err(ManualExecutionRepositoryError::Unavailable),
    };
    Ok(ManualExecutionEvent {
        id: parse_uuid(column(&row, "id")?)?,
        decision_record_id: parse_uuid(column(&row, "decision_record_id")?)?,
        plan_id: parse_uuid(column(&row, "plan_id")?)?,
        outcome: outcome_from_str(column(&row, "outcome")?)?,
        actual_amount: column::<Option<String>>(&row, "actual_amount")?
            .map(decode_actual_amount)
            .transpose()?,
        currency: column(&row, "currency")?,
        note: column(&row, "note")?,
        occurred_at: timestamp_from_micros(column(&row, "occurred_at_micros")?)?,
        recorded_at: timestamp_from_micros(column(&row, "recorded_at_micros")?)?,
        source,
    })
}

fn encode_actual_amount(value: Decimal) -> Result<String, ManualExecutionRepositoryError> {
    encode_amount(value).ok_or(ManualExecutionRepositoryError::Validation(
        decision_records::ManualExecutionValidationError::InvalidActualAmount,
    ))
}

fn decode_actual_amount(value: String) -> Result<Decimal, ManualExecutionRepositoryError> {
    let mut amount = decode_amount(&value).ok_or(ManualExecutionRepositoryError::Unavailable)?;
    amount.rescale(amount.normalize().scale().max(2));
    Ok(amount)
}

fn timestamp_to_micros(value: OffsetDateTime) -> Result<i64, ManualExecutionRepositoryError> {
    i64::try_from(value.unix_timestamp_nanos() / 1_000)
        .map_err(|_| ManualExecutionRepositoryError::Unavailable)
}

fn timestamp_from_micros(value: i64) -> Result<OffsetDateTime, ManualExecutionRepositoryError> {
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(value) * 1_000)
        .map_err(|_| ManualExecutionRepositoryError::Unavailable)
}

fn parse_uuid(value: String) -> Result<Uuid, ManualExecutionRepositoryError> {
    value
        .parse()
        .map_err(|_| ManualExecutionRepositoryError::Unavailable)
}

fn outcome_from_str(
    value: String,
) -> Result<ManualExecutionOutcome, ManualExecutionRepositoryError> {
    match value.as_str() {
        "executed" => Ok(ManualExecutionOutcome::Executed),
        "skipped" => Ok(ManualExecutionOutcome::Skipped),
        "partial" => Ok(ManualExecutionOutcome::Partial),
        _ => Err(ManualExecutionRepositoryError::Unavailable),
    }
}

fn outcome_to_str(value: ManualExecutionOutcome) -> &'static str {
    match value {
        ManualExecutionOutcome::Executed => "executed",
        ManualExecutionOutcome::Skipped => "skipped",
        ManualExecutionOutcome::Partial => "partial",
    }
}

fn column<'row, T>(row: &'row SqliteRow, name: &str) -> Result<T, ManualExecutionRepositoryError>
where
    T: sqlx::Decode<'row, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite>,
{
    row.try_get(name).map_err(map_sqlx_error)
}

fn map_sqlx_error(error: sqlx::Error) -> ManualExecutionRepositoryError {
    if error.as_database_error().is_some_and(|database_error| {
        database_error.is_unique_violation()
            || database_error
                .message()
                .contains("manual execution outcome already recorded")
    }) {
        ManualExecutionRepositoryError::AlreadyExists
    } else {
        tracing::warn!(error = %error, "manual execution SQLite operation failed");
        ManualExecutionRepositoryError::Unavailable
    }
}

#[cfg(test)]
mod tests {
    use decision_records::{
        CreateDecisionRecord, DecisionExecutionStatus, DecisionRecordRepository,
    };
    use investment_plans::{
        CreateInvestmentPlan, InvestmentPlanRepository, PlanExecutionConfiguration, ScheduleKind,
    };
    use serde_json::json;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::{SqliteDecisionRecordRepository, SqliteInvestmentPlanRepository, SqliteStorage};

    async fn repositories() -> (
        SqliteManualExecutionRepository,
        SqliteDecisionRecordRepository,
        SqliteInvestmentPlanRepository,
    ) {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .in_memory(true)
                    .foreign_keys(true),
            )
            .await
            .unwrap();
        let storage = SqliteStorage::from_pool(pool);
        storage.migrate().await.unwrap();
        (
            SqliteManualExecutionRepository::new(storage.pool().clone()),
            SqliteDecisionRecordRepository::new(storage.pool().clone()),
            SqliteInvestmentPlanRepository::new(storage.pool().clone()),
        )
    }

    async fn create_decision(
        decisions: &SqliteDecisionRecordRepository,
        plans: &SqliteInvestmentPlanRepository,
    ) -> decision_records::DecisionRecord {
        let plan = plans
            .create(CreateInvestmentPlan {
                name: "Manual journal plan".to_owned(),
                symbol: "VOO".to_owned(),
                base_contribution: Decimal::new(1000, 0),
                currency: "USD".to_owned(),
                schedule_kind: ScheduleKind::Monthly,
                schedule_day: 15,
                schedule_days: vec![15],
                policy: None,
                execution_configuration: PlanExecutionConfiguration::default(),
                max_single_execution: Decimal::new(1500, 0),
            })
            .await
            .unwrap();
        decisions
            .create(CreateDecisionRecord {
                plan_id: plan.id,
                symbol: plan.symbol,
                currency: plan.currency,
                execution_status: DecisionExecutionStatus::Due,
                planned_contribution: Some("1000.00".to_owned()),
                execution_snapshot: json!({"status": "due"}),
                fundamental_snapshot: json!({"used": false}),
                trend_snapshot: json!({"used": false}),
                sentiment_snapshot: None,
                decision_snapshot: json!({"action": "standard"}),
                policy_evidence: None,
                broker_order_request: None,
                broker_order_ack: None,
                summary: "Invest the scheduled amount.".to_owned(),
            })
            .await
            .unwrap()
    }

    fn input(
        id: Uuid,
        decision_record_id: Uuid,
        outcome: ManualExecutionOutcome,
        actual_amount: Option<Decimal>,
    ) -> CreateManualExecutionEvent {
        CreateManualExecutionEvent {
            id,
            decision_record_id,
            outcome,
            actual_amount,
            note: Some("reported from broker history".to_owned()),
            occurred_at: OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
        }
    }

    #[tokio::test]
    async fn appends_and_lists_events_without_mutating_the_decision_record() {
        let (journal, decisions, plans) = repositories().await;
        let decision = create_decision(&decisions, &plans).await;
        let before = decisions.get(decision.id).await.unwrap();

        let first = journal
            .append(input(
                Uuid::from_u128(101),
                decision.id,
                ManualExecutionOutcome::Partial,
                Some(Decimal::new(75000, 2)),
            ))
            .await
            .unwrap();
        assert_eq!(first.plan_id, decision.plan_id);
        assert_eq!(first.currency, "USD");
        assert_eq!(first.source, ManualExecutionSource::UserReported);
        assert_eq!(
            journal.list_by_decision(decision.id).await.unwrap(),
            vec![first]
        );
        assert_eq!(decisions.get(decision.id).await.unwrap(), before);

        plans.delete(decision.plan_id).await.unwrap();
        assert!(journal
            .list_by_decision(decision.id)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn rejects_duplicate_ids_missing_decisions_and_direct_mutation() {
        let (journal, decisions, plans) = repositories().await;
        let decision = create_decision(&decisions, &plans).await;
        let event_id = Uuid::from_u128(201);
        journal
            .append(input(
                event_id,
                decision.id,
                ManualExecutionOutcome::Executed,
                Some(Decimal::new(1000, 0)),
            ))
            .await
            .unwrap();

        assert_eq!(
            journal
                .append(input(
                    event_id,
                    decision.id,
                    ManualExecutionOutcome::Executed,
                    Some(Decimal::new(1000, 0)),
                ))
                .await,
            Err(ManualExecutionRepositoryError::AlreadyExists)
        );
        assert_eq!(
            journal
                .append(input(
                    Uuid::from_u128(202),
                    decision.id,
                    ManualExecutionOutcome::Skipped,
                    None,
                ))
                .await,
            Err(ManualExecutionRepositoryError::AlreadyExists)
        );
        assert_eq!(
            journal
                .append(input(
                    Uuid::from_u128(203),
                    Uuid::from_u128(999),
                    ManualExecutionOutcome::Skipped,
                    None,
                ))
                .await,
            Err(ManualExecutionRepositoryError::NotFound)
        );

        let update =
            sqlx::query("UPDATE manual_execution_events SET note = 'changed' WHERE id = ?1")
                .bind(event_id.to_string())
                .execute(&journal.pool)
                .await;
        let delete = sqlx::query("DELETE FROM manual_execution_events WHERE id = ?1")
            .bind(event_id.to_string())
            .execute(&journal.pool)
            .await;
        assert!(update.is_err());
        assert!(delete.is_err());
    }
}
