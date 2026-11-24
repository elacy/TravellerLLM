-- TravellerLLM initial PostgreSQL schema (authoritative game state + pipe-parity LLM orchestration)

CREATE TABLE IF NOT EXISTS sectors (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    abbreviation TEXT NOT NULL,
    sector_x INT NOT NULL,
    sector_y INT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb
);

CREATE TABLE IF NOT EXISTS systems (
    id UUID PRIMARY KEY,
    sector_id UUID NOT NULL REFERENCES sectors(id) ON DELETE CASCADE,
    hex TEXT NOT NULL,
    name TEXT NOT NULL,
    uwp TEXT NOT NULL,
    starport CHAR(1) NOT NULL,
    tech_level SMALLINT NOT NULL,
    law_level SMALLINT NOT NULL,
    trade_codes TEXT[] NOT NULL DEFAULT '{}',
    bases TEXT[] NOT NULL DEFAULT '{}',
    allegiance TEXT NOT NULL DEFAULT 'Im',
    travel_zone TEXT NOT NULL DEFAULT 'Green',
    gas_giants SMALLINT NOT NULL DEFAULT 0,
    global_x INT NOT NULL DEFAULT 0,
    global_y INT NOT NULL DEFAULT 0,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    UNIQUE(sector_id, hex)
);

CREATE INDEX IF NOT EXISTS idx_systems_name ON systems (LOWER(name));
CREATE INDEX IF NOT EXISTS idx_systems_coords ON systems (global_x, global_y);

CREATE TABLE IF NOT EXISTS items (
    id UUID PRIMARY KEY,
    pack_id TEXT NOT NULL,
    category TEXT NOT NULL,
    name TEXT NOT NULL,
    tl SMALLINT NOT NULL,
    cost_cr BIGINT NOT NULL,
    mass_kg DOUBLE PRECISION NOT NULL DEFAULT 0,
    law_restriction SMALLINT NOT NULL DEFAULT 0,
    stats JSONB NOT NULL DEFAULT '{}'::jsonb,
    tags TEXT[] NOT NULL DEFAULT '{}',
    UNIQUE(pack_id, name)
);

CREATE TABLE IF NOT EXISTS campaigns (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    imperial_year INT NOT NULL DEFAULT 1105,
    imperial_day SMALLINT NOT NULL DEFAULT 1,
    hour SMALLINT NOT NULL DEFAULT 8,
    minute SMALLINT NOT NULL DEFAULT 0,
    current_system_name TEXT NOT NULL DEFAULT 'Regina',
    current_system_uwp TEXT NOT NULL DEFAULT 'A788899-C',
    current_system_id UUID REFERENCES systems(id) ON DELETE SET NULL,
    referee_model TEXT NOT NULL,
    worker_model TEXT NOT NULL,
    hindsight_bank_id TEXT NOT NULL,
    rules_config JSONB NOT NULL DEFAULT '{}'::jsonb,
    story_ledger JSONB NOT NULL DEFAULT '{"who_knows_what":"","settled":"","people":"","threads":""}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS characters (
    id UUID PRIMARY KEY,
    campaign_id UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('player', 'crew', 'npc')),
    name TEXT NOT NULL,
    species TEXT NOT NULL DEFAULT 'Human',
    age SMALLINT NOT NULL DEFAULT 26,
    upp_max JSONB NOT NULL,
    upp_current JSONB NOT NULL,
    skills JSONB NOT NULL DEFAULT '{}'::jsonb,
    augments JSONB NOT NULL DEFAULT '[]'::jsonb,
    equipment JSONB NOT NULL DEFAULT '[]'::jsonb,
    skill_xp JSONB NOT NULL DEFAULT '{}'::jsonb,
    characteristic_xp JSONB NOT NULL DEFAULT '{}'::jsonb,
    general_xp INT NOT NULL DEFAULT 0,
    salary_cr BIGINT NOT NULL DEFAULT 0,
    loyalty INT NOT NULL DEFAULT 0,
    assigned_ship_id UUID,
    assigned_station TEXT,
    notes TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_characters_campaign ON characters (campaign_id);

CREATE TABLE IF NOT EXISTS accounts (
    id UUID PRIMARY KEY,
    campaign_id UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('personal', 'company')),
    name TEXT NOT NULL,
    owner_character_id UUID REFERENCES characters(id) ON DELETE SET NULL,
    balance_cr BIGINT NOT NULL DEFAULT 0,
    credit_limit_cr BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_accounts_campaign ON accounts (campaign_id);

CREATE TABLE IF NOT EXISTS transactions (
    id UUID PRIMARY KEY,
    campaign_id UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    account_id UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    imperial_date TEXT NOT NULL,
    amount_cr BIGINT NOT NULL,
    balance_after_cr BIGINT NOT NULL,
    category TEXT NOT NULL,
    description TEXT NOT NULL,
    counterparty TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_transactions_campaign ON transactions (campaign_id, created_at DESC);

CREATE TABLE IF NOT EXISTS ships (
    id UUID PRIMARY KEY,
    campaign_id UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    class_name TEXT NOT NULL,
    owner_account_id UUID REFERENCES accounts(id) ON DELETE SET NULL,
    spec JSONB NOT NULL,
    validation JSONB NOT NULL,
    hull_current INT NOT NULL,
    fuel_refined DOUBLE PRECISION NOT NULL DEFAULT 0,
    fuel_unrefined DOUBLE PRECISION NOT NULL DEFAULT 0,
    power_alloc JSONB NOT NULL DEFAULT '{}'::jsonb,
    crits JSONB NOT NULL DEFAULT '[]'::jsonb,
    cargo JSONB NOT NULL DEFAULT '[]'::jsonb,
    passengers JSONB NOT NULL DEFAULT '[]'::jsonb,
    mortgage JSONB,
    skipped_maintenance_periods INT NOT NULL DEFAULT 0,
    last_annual_overhaul_year INT NOT NULL DEFAULT 1105,
    docked_system_name TEXT NOT NULL DEFAULT 'Regina',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ships_campaign ON ships (campaign_id);

CREATE TABLE IF NOT EXISTS combats (
    id UUID PRIMARY KEY,
    campaign_id UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('personal', 'space')),
    status TEXT NOT NULL CHECK (status IN ('active', 'ended')),
    round INT NOT NULL DEFAULT 1,
    phase TEXT NOT NULL DEFAULT 'initiative',
    state JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS scheduled_events (
    id UUID PRIMARY KEY,
    campaign_id UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    due_total_minutes BIGINT NOT NULL,
    due_label TEXT NOT NULL,
    kind TEXT NOT NULL,
    title TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    completed BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_events_campaign_due ON scheduled_events (campaign_id, completed, due_total_minutes);

-- Conversation turns, turn transcript replay (pipe parity), hierarchical summaries, and audit logs
CREATE TABLE IF NOT EXISTS messages (
    id BIGSERIAL PRIMARY KEY,
    campaign_id UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    content TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    superseded BOOLEAN NOT NULL DEFAULT FALSE,
    model TEXT,
    input_tokens INT NOT NULL DEFAULT 0,
    output_tokens INT NOT NULL DEFAULT 0,
    cache_read_tokens INT NOT NULL DEFAULT 0,
    cache_creation_tokens INT NOT NULL DEFAULT 0,
    cost_usd DOUBLE PRECISION NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_messages_campaign ON messages (campaign_id, id ASC);

CREATE TABLE IF NOT EXISTS turn_legs (
    id BIGSERIAL PRIMARY KEY,
    campaign_id UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    assistant_message_id BIGINT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    leg_index INT NOT NULL,
    assistant_blocks JSONB NOT NULL,
    tool_results JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_turn_legs_msg ON turn_legs (assistant_message_id, leg_index ASC);

CREATE TABLE IF NOT EXISTS message_summaries (
    id BIGSERIAL PRIMARY KEY,
    campaign_id UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    level INT NOT NULL,
    block_index INT NOT NULL,
    from_message_id BIGINT NOT NULL,
    to_message_id BIGINT NOT NULL,
    summary_text TEXT NOT NULL,
    model TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(campaign_id, level, block_index)
);

CREATE TABLE IF NOT EXISTS state_audit_log (
    id BIGSERIAL PRIMARY KEY,
    campaign_id UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    diff JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_audit_campaign ON state_audit_log (campaign_id, id DESC);
