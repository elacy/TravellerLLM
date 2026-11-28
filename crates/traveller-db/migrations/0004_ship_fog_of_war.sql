-- MgT2e High Guard fog-of-war tracking for in-system starship traffic:
-- - Transponder-dark ships start UNDETECTED until spotted via active sensor sweep.
-- - Captain names are hidden unless broadcast/identified on comms.
-- - Hidden disposition is never shown; hostility is only shown once observed (firing, combat, or threat).

ALTER TABLE system_ships
    ADD COLUMN IF NOT EXISTS detected BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS detection_level TEXT NOT NULL DEFAULT 'transponder',
    ADD COLUMN IF NOT EXISTS captain_known BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS hostility_observed BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS observed_status TEXT NOT NULL DEFAULT 'transponder_active';

UPDATE system_ships
SET detected = transponder_active,
    detection_level = CASE WHEN transponder_active THEN 'transponder' ELSE 'undetected' END,
    captain_known = FALSE,
    hostility_observed = (status = 'in_combat'),
    observed_status = CASE
        WHEN status = 'destroyed' THEN 'destroyed'
        WHEN status = 'in_combat' THEN 'hostile'
        WHEN disposition = 'distress' AND transponder_active THEN 'distress_beacon'
        WHEN transponder_active THEN 'transponder_active'
        ELSE 'running_dark'
    END;
