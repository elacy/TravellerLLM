use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use traveller_rules::calendar::ImperialDate;
use traveller_rules::character::{Augment, Character, CharacterKind, Upp};
use traveller_rules::finance::AccountKind;
use traveller_rules::ship::{Mortgage, ShipSpec, ShipValidationReport};
use uuid::Uuid;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StoryLedger {
    #[serde(default)]
    pub who_knows_what: String,
    #[serde(default)]
    pub settled: String,
    #[serde(default)]
    pub people: String,
    #[serde(default)]
    pub threads: String,
}

impl StoryLedger {
    pub fn has_content(&self) -> bool {
        !self.who_knows_what.trim().is_empty()
            || !self.settled.trim().is_empty()
            || !self.people.trim().is_empty()
            || !self.threads.trim().is_empty()
    }

    pub fn render_xml(&self) -> String {
        format!(
            "<story_ledger>\nThe current narrative state of play (update with update_ledger when facts change).\n\n## Who knows what\n{}\n\n## Settled facts\n{}\n\n## People\n{}\n\n## Open threads\n{}\n</story_ledger>",
            if self.who_knows_what.trim().is_empty() { "(nothing yet)" } else { self.who_knows_what.trim() },
            if self.settled.trim().is_empty() { "(nothing yet)" } else { self.settled.trim() },
            if self.people.trim().is_empty() { "(nothing yet)" } else { self.people.trim() },
            if self.threads.trim().is_empty() { "(nothing yet)" } else { self.threads.trim() },
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignRecord {
    pub id: Uuid,
    pub name: String,
    pub date: ImperialDate,
    pub current_system_name: String,
    pub current_system_uwp: String,
    pub referee_model: String,
    pub worker_model: String,
    pub hindsight_bank_id: String,
    pub story_ledger: StoryLedger,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipRecord {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub name: String,
    pub class_name: String,
    pub owner_account_id: Option<Uuid>,
    pub spec: ShipSpec,
    pub validation: ShipValidationReport,
    pub hull_current: i32,
    pub fuel_refined: f64,
    pub fuel_unrefined: f64,
    pub crits: Vec<Value>,
    pub cargo: Vec<Value>,
    pub passengers: Vec<Value>,
    pub mortgage: Option<Mortgage>,
    pub skipped_maintenance_periods: i32,
    pub last_annual_overhaul_year: i32,
    pub docked_system_name: String,
    #[serde(default)]
    pub is_docked: bool,
    #[serde(default)]
    pub docked_at_total_minutes: Option<i64>,
    #[serde(default)]
    pub berthing_billed_days: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountRecord {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub kind: AccountKind,
    pub name: String,
    pub owner_character_id: Option<Uuid>,
    pub balance_cr: i64,
    pub credit_limit_cr: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRecord {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub account_id: Uuid,
    pub imperial_date: String,
    pub amount_cr: i64,
    pub balance_after_cr: i64,
    pub category: String,
    pub description: String,
    pub counterparty: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRecord {
    pub id: i64,
    pub campaign_id: Uuid,
    pub role: String,
    pub content: String,
    pub content_hash: String,
    pub superseded: bool,
    pub model: Option<String>,
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub cache_read_tokens: i32,
    pub cache_creation_tokens: i32,
    pub cost_usd: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnLegRecord {
    pub id: i64,
    pub assistant_message_id: i64,
    pub leg_index: i32,
    pub assistant_blocks: Value,
    pub tool_results: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSummaryRecord {
    pub id: i64,
    pub level: i32,
    pub block_index: i32,
    pub from_message_id: i64,
    pub to_message_id: i64,
    pub summary_text: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledEventRecord {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub due_total_minutes: i64,
    pub due_label: String,
    pub kind: String,
    pub title: String,
    pub payload: Value,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorRecord {
    pub id: Uuid,
    pub name: String,
    pub abbreviation: String,
    pub sector_x: i32,
    pub sector_y: i32,
    pub system_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemRecord {
    pub id: Uuid,
    pub sector_id: Uuid,
    pub sector_name: String,
    pub hex: String,
    pub name: String,
    pub uwp: String,
    pub starport: String,
    pub tech_level: i16,
    pub law_level: i16,
    pub trade_codes: Vec<String>,
    pub bases: Vec<String>,
    pub allegiance: String,
    pub travel_zone: String,
    pub gas_giants: i16,
    pub global_x: i32,
    pub global_y: i32,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemRecord {
    pub id: Uuid,
    pub pack_id: String,
    pub category: String,
    pub name: String,
    pub tl: i16,
    pub cost_cr: i64,
    pub mass_kg: f64,
    pub law_restriction: i16,
    pub stats: Value,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatRecord {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub kind: String,
    pub status: String,
    pub round: i32,
    pub phase: String,
    pub state: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterRow {
    pub character: Character,
    pub equipment: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewCharacterInput {
    pub kind: CharacterKind,
    pub name: String,
    #[serde(default = "default_species")]
    pub species: String,
    #[serde(default = "default_age")]
    pub age: u16,
    #[serde(default)]
    pub upp: Upp,
    #[serde(default)]
    pub skills: std::collections::BTreeMap<String, i32>,
    #[serde(default)]
    pub augments: Vec<Augment>,
    #[serde(default)]
    pub salary_cr: i64,
    #[serde(default)]
    pub assigned_ship_id: Option<Uuid>,
    #[serde(default)]
    pub assigned_station: Option<String>,
    #[serde(default)]
    pub notes: String,
}

fn default_species() -> String {
    "Human".to_string()
}

fn default_age() -> u16 {
    26
}

// ── Campaign Modules (Knowledge Files for Hindsight + NPCs/Ships/Events) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleKnowledgeFile {
    pub filename: String,
    #[serde(default)]
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleEventSpec {
    pub title: String,
    #[serde(default = "default_event_kind")]
    pub kind: String,
    #[serde(default)]
    pub days_from_now: i64,
    #[serde(default)]
    pub hours_from_now: i64,
}

fn default_event_kind() -> String {
    "story".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleRecord {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub starting_system_name: Option<String>,
    pub starting_system_uwp: Option<String>,
    pub referee_brief: String,
    pub knowledge_files: Vec<ModuleKnowledgeFile>,
    pub npcs: Vec<NewCharacterInput>,
    pub ships: Vec<Value>,
    pub events: Vec<ModuleEventSpec>,
    pub tags: Vec<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertModuleInput {
    pub slug: String,
    pub name: String,
    #[serde(default = "default_mod_ver")]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub starting_system_name: Option<String>,
    #[serde(default)]
    pub starting_system_uwp: Option<String>,
    #[serde(default)]
    pub referee_brief: String,
    #[serde(default)]
    pub knowledge_files: Vec<ModuleKnowledgeFile>,
    #[serde(default)]
    pub npcs: Vec<NewCharacterInput>,
    #[serde(default)]
    pub ships: Vec<Value>,
    #[serde(default)]
    pub events: Vec<ModuleEventSpec>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "default_mod_source")]
    pub source: String,
}

fn default_mod_ver() -> String {
    "1.0".to_string()
}

fn default_mod_source() -> String {
    "custom".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignModuleRecord {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub module_id: Uuid,
    pub module_slug: String,
    pub module_name: String,
    pub referee_brief: String,
    pub knowledge_file_count: usize,
    pub hindsight_chunks_ingested: i32,
    pub npcs_spawned: i32,
    pub ships_spawned: i32,
    pub events_spawned: i32,
    pub loaded_at: DateTime<Utc>,
}

// ── In-System Starship Traffic ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemShipRecord {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub system_name: String,
    pub name: String,
    pub ship_class: String,
    pub role: String,
    pub tonnage: i32,
    pub tl: i16,
    pub hull_max: i32,
    pub hull_current: i32,
    pub armor: i32,
    pub thrust: i16,
    pub jump_rating: i16,
    pub sensor_grade: String,
    pub stealth_dm: i16,
    pub transponder_active: bool,
    /// GM-only true disposition (NEVER sent raw to the player API/UI!).
    pub disposition: String,
    pub orbit_location: String,
    /// GM-only captain name (only exposed to player when `captain_known == true`).
    pub captain_name: String,
    pub faction: String,
    pub cargo_hint: String,
    pub comms_log: Vec<Value>,
    pub status: String,
    /// Whether the player's sensors/passive scope have spotted this vessel.
    #[serde(default = "default_true_db")]
    pub detected: bool,
    /// Highest achieved sensor knowledge: "undetected" | "transponder" | "minimal" | "limited" | "full" | "pinpoint_lock"
    #[serde(default = "default_detection_level")]
    pub detection_level: String,
    /// Whether the captain's name has been broadcast or revealed on comms.
    #[serde(default)]
    pub captain_known: bool,
    /// Whether hostile action (firing, weapon lock/combat, pirate threat) has actually been observed.
    #[serde(default)]
    pub hostility_observed: bool,
    /// Player-observable status: "transponder_active" | "running_dark" | "distress_beacon" | "comms_open" | "weapons_hot" | "hostile" | "destroyed"
    #[serde(default = "default_observed_status")]
    pub observed_status: String,
}

fn default_true_db() -> bool {
    true
}

fn default_detection_level() -> String {
    "transponder".to_string()
}

fn default_observed_status() -> String {
    "transponder_active".to_string()
}

/// Fog-of-war redacted view of an in-system vessel returned to the player client.
/// Never leaks undetected ships, hidden `disposition`, unbroadcast `captain_name`, or unscanned specs!
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerVisibleSystemShip {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub system_name: String,
    pub display_name: String,
    pub ship_class: Option<String>,
    pub tonnage: Option<i32>,
    pub tl: Option<i16>,
    pub hull_max: Option<i32>,
    pub hull_current: Option<i32>,
    pub armor: Option<i32>,
    pub thrust: Option<i16>,
    pub jump_rating: Option<i16>,
    pub sensor_grade: Option<String>,
    pub stealth_dm: Option<i16>,
    pub transponder_active: bool,
    pub orbit_location: String,
    pub captain_name: Option<String>,
    pub faction: Option<String>,
    pub sensor_notes: Option<String>,
    pub detection_level: String,
    pub hostility_observed: bool,
    pub observed_status: String,
    pub comms_log: Vec<Value>,
    pub status: String,
}

pub fn detection_level_rank(level: &str) -> u8 {
    match level {
        "undetected" => 0,
        "minimal" => 1,
        "transponder" => 2,
        "limited" => 3,
        "full" => 4,
        "pinpoint_lock" => 5,
        _ => 1,
    }
}

impl SystemShipRecord {
    /// Short stable sensor track ID (e.g. `A3F2`) used for unidentified dark contacts.
    pub fn track_code(&self) -> String {
        let hex = self.id.simple().to_string();
        hex[..4].to_uppercase()
    }

    /// Project this vessel into what the player's ship sensors and comms have actually observed.
    /// Returns `None` if the vessel is still undetected (e.g. transponder-dark and not yet spotted).
    pub fn to_player_view(&self) -> Option<PlayerVisibleSystemShip> {
        if !self.detected {
            return None;
        }
        let rank = detection_level_rank(&self.detection_level);
        let has_limited = rank >= detection_level_rank("limited");
        let has_full = rank >= detection_level_rank("full");

        let display_name = if self.transponder_active {
            self.name.clone()
        } else if has_limited {
            format!("Unidentified ~{}t Contact [{}]", self.tonnage, self.track_code())
        } else {
            format!("Unidentified Contact [{}]", self.track_code())
        };

        let ship_class = if self.transponder_active || has_limited {
            Some(self.ship_class.clone())
        } else {
            None
        };
        let tonnage = if self.transponder_active || has_limited {
            Some(self.tonnage)
        } else {
            None
        };
        let faction = if self.transponder_active || self.captain_known {
            Some(self.faction.clone())
        } else {
            None
        };
        let captain_name = if self.captain_known {
            Some(self.captain_name.clone())
        } else {
            None
        };

        let observed_status = if self.status == "destroyed" {
            "destroyed".to_string()
        } else if self.hostility_observed || self.status == "in_combat" {
            "hostile".to_string()
        } else if self.observed_status == "hostile" && !self.hostility_observed {
            if self.transponder_active {
                "transponder_active".to_string()
            } else {
                "running_dark".to_string()
            }
        } else {
            self.observed_status.clone()
        };

        Some(PlayerVisibleSystemShip {
            id: self.id,
            campaign_id: self.campaign_id,
            system_name: self.system_name.clone(),
            display_name,
            ship_class,
            tonnage,
            tl: if has_full { Some(self.tl) } else { None },
            hull_max: if has_full { Some(self.hull_max) } else { None },
            hull_current: if has_full { Some(self.hull_current) } else { None },
            armor: if has_full { Some(self.armor) } else { None },
            thrust: if has_limited { Some(self.thrust) } else { None },
            jump_rating: if has_limited { Some(self.jump_rating) } else { None },
            sensor_grade: if has_full { Some(self.sensor_grade.clone()) } else { None },
            stealth_dm: if has_full { Some(self.stealth_dm) } else { None },
            transponder_active: self.transponder_active,
            orbit_location: self.orbit_location.clone(),
            captain_name,
            faction,
            sensor_notes: if has_full {
                Some(self.cargo_hint.clone())
            } else {
                None
            },
            detection_level: self.detection_level.clone(),
            hostility_observed: self.hostility_observed || self.status == "in_combat",
            observed_status,
            comms_log: self.comms_log.clone(),
            status: self.status.clone(),
        })
    }
}
