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
    pub disposition: String,
    pub orbit_location: String,
    pub captain_name: String,
    pub faction: String,
    pub cargo_hint: String,
    pub comms_log: Vec<Value>,
    pub status: String,
}
