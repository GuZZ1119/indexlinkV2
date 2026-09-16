-- Append-only journal of user-reported outcomes for immutable decision records.
--
-- Amounts reuse the local fixed-width exact-decimal representation. Timestamps
-- are UTC Unix microseconds so offsets cannot be lost or reinterpreted.

CREATE TABLE manual_execution_events (
    id TEXT PRIMARY KEY NOT NULL,
    decision_record_id TEXT NOT NULL REFERENCES decision_records(id) ON DELETE CASCADE,
    plan_id TEXT NOT NULL REFERENCES investment_plans(id) ON DELETE CASCADE,
    outcome TEXT NOT NULL,
    actual_amount TEXT,
    currency TEXT NOT NULL,
    note TEXT,
    occurred_at_micros INTEGER NOT NULL,
    recorded_at_micros INTEGER NOT NULL,
    source TEXT NOT NULL DEFAULT 'user_reported',

    CONSTRAINT manual_execution_events_outcome_check
        CHECK (outcome IN ('executed', 'skipped', 'partial')),
    CONSTRAINT manual_execution_events_amount_check
        CHECK (
            (outcome = 'skipped' AND actual_amount IS NULL)
            OR (
                outcome IN ('executed', 'partial')
                AND actual_amount GLOB '[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9].[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]'
                AND actual_amount > '000000000000.00000000'
            )
        ),
    CONSTRAINT manual_execution_events_currency_check
        CHECK (currency GLOB '[A-Z][A-Z][A-Z]'),
    CONSTRAINT manual_execution_events_note_check
        CHECK (
            note IS NULL
            OR (note = trim(note) AND length(note) BETWEEN 1 AND 500)
        ),
    CONSTRAINT manual_execution_events_source_check
        CHECK (source = 'user_reported')
);

CREATE INDEX manual_execution_events_decision_idx
    ON manual_execution_events (decision_record_id, recorded_at_micros ASC, id ASC);

CREATE INDEX manual_execution_events_plan_idx
    ON manual_execution_events (plan_id, recorded_at_micros DESC, id DESC);

CREATE TRIGGER manual_execution_events_reject_update
BEFORE UPDATE ON manual_execution_events
BEGIN
    SELECT RAISE(ABORT, 'manual execution events are append-only');
END;

-- Explicit event deletion is forbidden while its parent decision exists. A
-- deliberate plan/decision cascade remains compatible with the existing local
-- data-deletion contract.
CREATE TRIGGER manual_execution_events_reject_direct_delete
BEFORE DELETE ON manual_execution_events
WHEN EXISTS (
    SELECT 1 FROM investment_plans WHERE id = OLD.plan_id
)
BEGIN
    SELECT RAISE(ABORT, 'manual execution events are append-only');
END;
