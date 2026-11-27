-- Enforce strictly ONE player character per campaign, add Campaign Modules (with Hindsight knowledge files & NPCs),
-- and add persistent In-System Starship Traffic encounters.

-- 1. Safely demote any accidental duplicate 'player' characters (keeping the earliest per campaign)
WITH ranked_players AS (
    SELECT id,
           ROW_NUMBER() OVER (PARTITION BY campaign_id ORDER BY created_at ASC, id ASC) AS rn
    FROM characters
    WHERE kind = 'player'
)
UPDATE characters
SET kind = 'crew'
WHERE id IN (SELECT id FROM ranked_players WHERE rn > 1);

CREATE UNIQUE INDEX IF NOT EXISTS idx_characters_one_player_per_campaign
    ON characters (campaign_id)
    WHERE kind = 'player';

-- 2. Reusable Campaign Modules (knowledge files for Hindsight + pre-built NPCs, ships, events)
CREATE TABLE IF NOT EXISTS modules (
    id UUID PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    version TEXT NOT NULL DEFAULT '1.0',
    description TEXT NOT NULL DEFAULT '',
    author TEXT NOT NULL DEFAULT '',
    starting_system_name TEXT,
    starting_system_uwp TEXT,
    referee_brief TEXT NOT NULL DEFAULT '',
    knowledge_files JSONB NOT NULL DEFAULT '[]'::jsonb,
    npcs JSONB NOT NULL DEFAULT '[]'::jsonb,
    ships JSONB NOT NULL DEFAULT '[]'::jsonb,
    events JSONB NOT NULL DEFAULT '[]'::jsonb,
    tags TEXT[] NOT NULL DEFAULT '{}',
    source TEXT NOT NULL DEFAULT 'pack',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS campaign_modules (
    id UUID PRIMARY KEY,
    campaign_id UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    module_id UUID NOT NULL REFERENCES modules(id) ON DELETE CASCADE,
    hindsight_chunks_ingested INT NOT NULL DEFAULT 0,
    npcs_spawned INT NOT NULL DEFAULT 0,
    ships_spawned INT NOT NULL DEFAULT 0,
    events_spawned INT NOT NULL DEFAULT 0,
    loaded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(campaign_id, module_id)
);

CREATE INDEX IF NOT EXISTS idx_campaign_modules_campaign ON campaign_modules (campaign_id);

-- 3. Persistent In-System Starship Traffic (generated on Jump / Scan, interactive via Hail / Scan / Trade / Combat)
CREATE TABLE IF NOT EXISTS system_ships (
    id UUID PRIMARY KEY,
    campaign_id UUID NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    system_name TEXT NOT NULL,
    name TEXT NOT NULL,
    ship_class TEXT NOT NULL,
    role TEXT NOT NULL,
    tonnage INT NOT NULL DEFAULT 200,
    tl SMALLINT NOT NULL DEFAULT 12,
    hull_max INT NOT NULL DEFAULT 80,
    hull_current INT NOT NULL DEFAULT 80,
    armor INT NOT NULL DEFAULT 2,
    thrust SMALLINT NOT NULL DEFAULT 2,
    jump_rating SMALLINT NOT NULL DEFAULT 1,
    sensor_grade TEXT NOT NULL DEFAULT 'civilian',
    stealth_dm SMALLINT NOT NULL DEFAULT 0,
    transponder_active BOOLEAN NOT NULL DEFAULT TRUE,
    disposition TEXT NOT NULL DEFAULT 'neutral',
    orbit_location TEXT NOT NULL DEFAULT 'Mainworld Orbit',
    captain_name TEXT NOT NULL DEFAULT 'Unknown Captain',
    faction TEXT NOT NULL DEFAULT 'Independent',
    cargo_hint TEXT NOT NULL DEFAULT '',
    comms_log JSONB NOT NULL DEFAULT '[]'::jsonb,
    status TEXT NOT NULL DEFAULT 'in_system',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_system_ships_campaign_sys ON system_ships (campaign_id, system_name);
