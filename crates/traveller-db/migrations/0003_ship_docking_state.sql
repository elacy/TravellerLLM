-- Track whether each ship is actually docked at a starport, when it docked,
-- and how many days of berthing have already been covered/billed so players
-- cannot double-pay docking fees and extended stays automatically accrue daily fees.
ALTER TABLE ships
    ADD COLUMN IF NOT EXISTS is_docked BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS docked_at_total_minutes BIGINT,
    ADD COLUMN IF NOT EXISTS berthing_billed_days INTEGER NOT NULL DEFAULT 0;
