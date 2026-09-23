-- A scheduled decision represents one plan occurrence and can receive only one
-- user-reported terminal outcome. Existing histories are preserved unchanged;
-- this trigger only prevents additional events from being appended.

CREATE TRIGGER manual_execution_events_reject_second_outcome
BEFORE INSERT ON manual_execution_events
WHEN EXISTS (
    SELECT 1
    FROM manual_execution_events
    WHERE decision_record_id = NEW.decision_record_id
)
BEGIN
    SELECT RAISE(ABORT, 'manual execution outcome already recorded');
END;
