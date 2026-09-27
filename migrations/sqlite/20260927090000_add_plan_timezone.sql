-- Freeze the calendar boundary used by each plan.
-- Existing plans keep their historical UTC semantics; new plans persist the
-- browser-selected IANA timezone supplied through the validated API boundary.

ALTER TABLE plan_execution_configurations
    ADD COLUMN timezone TEXT NOT NULL DEFAULT 'UTC'
    CHECK (length(timezone) BETWEEN 1 AND 64);
