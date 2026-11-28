use anyhow::{Context, Result};
use rand::rng;
use serde_json::{json, Value};
use traveller_db::Db;
use traveller_rules::calendar::{compute_advance, roll_jump_duration_hours};
use traveller_rules::character::{Characteristic, CharacterKind};
use traveller_rules::combat_personal::{
    execute_personal_ai_turn, next_personal_round, resolve_personal_attack, start_personal_combat,
    PersonalAttackInput, PersonalCombatState, PersonalCombatant, PersonalRangeBand, PersonalWeapon,
    ReactionKind,
};
use traveller_rules::combat_space::{
    acquire_sensor_lock, advance_space_round, allocate_maneuver_thrust, ew_jam_missiles,
    execute_space_ai_turn, launch_missile_salvo, resolve_space_attack, start_space_combat,
    SpaceCombatShip, SpaceCombatState, SpaceRangeBand, SpaceWeaponAttackInput,
};
use traveller_rules::commerce::{check_item_availability, AvailabilityInput, LegalityCategory};
use traveller_rules::crew::generate_hiring_hall;
use traveller_rules::dice::{resolve_task, roll_d66, roll_nd6, Difficulty, ModifierItem, RollMode};
use traveller_rules::finance::AccountKind;
use traveller_rules::sensors::{resolve_sensor_detection, SensorContext, SensorDetectionInput};
use traveller_rules::ship::{
    berthing_extra_day_rate, calculate_accrued_berthing_fee, calculate_refuel_quote,
    compute_running_costs, fleet_max_jump_rating, jump_fuel_for_distance, roll_berthing_fee,
    roll_poor_maintenance, validate_ship, RefuelMode, SensorGrade,
};
use traveller_rules::trade::{compute_fleet_max_broker, generate_world_market, roll_speculative_sale};
use traveller_rules::traffic::generate_system_traffic;
use traveller_rules::universe::{
    build_spatial_memory_scope, hex_distance, parse_uwp, plan_jump_route, RouteNode,
};
use uuid::Uuid;

use crate::hindsight::HindsightClient;

/// Stable tool definitions (always in identical order so Anthropic prompt cache stays 100% hot).
pub fn tool_specs(hindsight_enabled: bool) -> Vec<Value> {
    let mut specs = vec![
        json!({
            "name": "roll_task",
            "description": "Roll a Mongoose Traveller 2e skill/characteristic task check for a character. Automatically applies wounded characteristic DMs, skill level (or -3 + Jack-of-All-Trades), Wafer Jack / Neural Jack + Expert software bonuses, Skill Augmentations, and optionally awards per-skill activity XP.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "character_id": {"type": "string", "description": "UUID of the character making the check."},
                    "skill": {"type": "string", "description": "Skill name (e.g. 'Pilot (Spacecraft)', 'Astrogation', 'Gun Combat (Slug)'). Omit for raw characteristic check."},
                    "characteristic": {"type": "string", "enum": ["STR", "DEX", "END", "INT", "EDU", "SOC", "PSI"]},
                    "difficulty": {"type": "string", "enum": ["simple", "easy", "routine", "average", "difficult", "very_difficult", "formidable", "impossible"], "default": "average"},
                    "mode": {"type": "string", "enum": ["normal", "boon", "bane"], "default": "normal"},
                    "extra_dm": {"type": "integer", "default": 0, "description": "Situational / equipment / task-chain DM."},
                    "reason": {"type": "string", "description": "Short label for what is being attempted."},
                    "award_activity_xp": {"type": "boolean", "default": true, "description": "Record +1 per-skill activity XP tick if meaningful check."}
                },
                "required": ["character_id", "characteristic"]
            }
        }),
        json!({
            "name": "roll_dice",
            "description": "Roll raw Traveller dice (1D6, 2D6, 3D6, D66, etc.) for random encounters, damage, or book tables.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "kind": {"type": "string", "enum": ["nd6", "d66"], "default": "nd6"},
                    "count": {"type": "integer", "default": 2},
                    "dm": {"type": "integer", "default": 0},
                    "reason": {"type": "string"}
                }
            }
        }),
        json!({
            "name": "sensor_check",
            "description": "Resolve a High Guard 2024 sensor detection or stealthed re-acquisition check using exact observer sensor grade, TL difference, target emissions (M-Drive thrust, power plant, active sensors, transponder/comms), and target stealth/armor modifiers. IMPORTANT: always pass target_stealth_armor_dm (e.g. -2, -4, -6) whenever the target has stealth coating, stealth hull, or stealth armor.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "context": {"type": "string", "enum": ["space_initial_detection", "space_stealthed_reacquire", "personal_ground"], "default": "space_initial_detection"},
                    "operator_character_id": {"type": "string"},
                    "observer_sensor_grade": {"type": "string", "enum": ["basic", "civilian", "military", "improved", "advanced"], "default": "civilian"},
                    "observer_tl": {"type": "integer", "default": 12},
                    "observer_extended_array": {"type": "boolean", "default": false},
                    "observer_active_sensors": {"type": "boolean", "default": true},
                    "target_tl": {"type": "integer", "default": 12},
                    "target_active_sensors": {"type": "boolean", "default": false},
                    "target_thrust_applied": {"type": "integer", "default": 0},
                    "target_power_plant_active": {"type": "boolean", "default": true},
                    "target_transponder_or_comms": {"type": "boolean", "default": false},
                    "target_stealth_armor_dm": {"type": "integer", "default": 0, "description": "Stealth coating (-2/-4/-6) or personal armor stealth modifier."},
                    "target_fired_weapons": {"type": "boolean", "default": false},
                    "target_crit_damage_severity": {"type": "integer", "default": 0},
                    "situational_dm": {"type": "integer", "default": 0}
                },
                "required": ["observer_sensor_grade"]
            }
        }),
        json!({
            "name": "explore_universe",
            "description": "Search Charted Space systems (all 512 sectors), inspect world UWP/trade codes/starport/gas giants, plot a Jump-1..6 A* route, or execute a jump to a destination system (burning jump fuel, advancing 148 + 6D6 hours, and automatically sweeping in-system starship traffic on arrival).",
            "input_schema": {
                "type": "object",
                "properties": {
                    "action": {"type": "string", "enum": ["search", "route", "jump_to"]},
                    "query": {"type": "string", "description": "World/sector name or hex for 'search'."},
                    "trade_code": {"type": "string", "description": "Optional 2-letter trade code filter (e.g. 'Hi', 'In', 'Ri', 'Ag')."},
                    "from_system": {"type": "string", "description": "Origin system name (defaults to campaign current system)."},
                    "to_system": {"type": "string", "description": "Destination system name for 'route' or 'jump_to'."},
                    "jump_rating": {"type": "integer", "default": 1},
                    "ship_id": {"type": "string", "description": "Optional ship UUID for 'jump_to' fuel deduction."}
                },
                "required": ["action"]
            }
        }),
        json!({
            "name": "system_traffic",
            "description": "Inspect or interact with starships currently in the campaign's star system: list all ships in-system, sweep/refresh system traffic, hail a vessel over comms, run a High Guard sensor scan against a specific vessel's emissions/stealth, engage a vessel in High Guard Space Combat, or update its disposition/status.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "action": {"type": "string", "enum": ["list", "scan_system", "hail", "sensor_scan", "engage", "update_status"]},
                    "vessel_id": {"type": "string", "description": "UUID of the in-system vessel for hail/sensor_scan/engage/update_status."},
                    "message": {"type": "string", "description": "Optional comms message when hailing a ship."},
                    "disposition": {"type": "string", "enum": ["friendly", "neutral", "cautious", "hostile", "distress"]},
                    "status": {"type": "string", "enum": ["in_system", "docked", "departed", "boarded", "destroyed"]},
                    "range_band": {"type": "string", "enum": ["adjacent", "close", "short", "medium", "long", "very_long", "distant"], "default": "long"}
                },
                "required": ["action"]
            }
        }),
        json!({
            "name": "trade_ops",
            "description": "Core 2022 Trade System: generate a world's Freight, Passengers, Mail, and Speculative Trade market; buy speculative goods into a ship's cargo hold (deducting from Company account); or sell cargo from the hold on the current world.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "action": {"type": "string", "enum": ["market", "buy_cargo", "sell_cargo"]},
                    "dest_system": {"type": "string", "description": "Target system name for freight/passenger calculation."},
                    "parsecs": {"type": "integer", "default": 1},
                    "broker_skill": {"type": "integer", "default": 1},
                    "steward_skill": {"type": "integer", "default": 0},
                    "include_illegal": {"type": "boolean", "default": false},
                    "ship_id": {"type": "string"},
                    "d66": {"type": "integer", "description": "D66 code of trade good (11..65) for buy_cargo or sell_cargo."},
                    "good_name": {"type": "string"},
                    "tons": {"type": "integer"},
                    "unit_price_cr": {"type": "integer"}
                },
                "required": ["action"]
            }
        }),
        json!({
            "name": "shop_market",
            "description": "Search the complete MgT2e item catalog (CSC 2023 / Core 2022 / Vehicles / Robots), run exact CSC 2023 Availability & Black Market checks with Law Enforcement response, and purchase items from Personal or Company accounts into character inventory.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "action": {"type": "string", "enum": ["search", "check_availability", "purchase"]},
                    "query": {"type": "string"},
                    "category": {"type": "string", "description": "weapon, armor, augment, software, electronics, medical, vehicle, robot_drone, equipment"},
                    "max_tl": {"type": "integer"},
                    "item_id": {"type": "string"},
                    "black_market": {"type": "boolean", "default": false},
                    "willing_price_multiplier": {"type": "integer", "default": 1},
                    "skill_dm": {"type": "integer", "default": 0},
                    "stat_dm": {"type": "integer", "default": 0},
                    "account_id": {"type": "string"},
                    "character_id": {"type": "string"},
                    "quantity": {"type": "integer", "default": 1},
                    "unit_price_cr": {"type": "integer"}
                },
                "required": ["action"]
            }
        }),
        json!({
            "name": "hire_crew",
            "description": "Generate Starport Hiring Hall crew candidates (using MgT2e lifepath careers & Core 2022 salaries) or hire a candidate onto a ship station (automatically adding their monthly salary to 4-week payroll).",
            "input_schema": {
                "type": "object",
                "properties": {
                    "action": {"type": "string", "enum": ["hiring_hall", "hire"]},
                    "roles": {"type": "array", "items": {"type": "string"}, "description": "e.g. ['Pilot', 'Engineer', 'Gunner', 'Medic']"},
                    "name": {"type": "string"},
                    "role": {"type": "string"},
                    "salary_cr": {"type": "integer"},
                    "ship_id": {"type": "string"}
                },
                "required": ["action"]
            }
        }),
        json!({
            "name": "port_ops",
            "description": "Starport & fleet operations: dock at the current system's starport (pays the Core 2022 1D initial berthing fee covering days 1–6; extended stay beyond 6 days is billed automatically by advance_time), undock to leave the starport into orbit, or refuel (refined Cr500/t at Starport A/B while docked, unrefined Cr100/t at A–D while docked, free gas giant skimming with fuel scoops while undocked, or onboard fuel processor refining).",
            "input_schema": {
                "type": "object",
                "properties": {
                    "action": {"type": "string", "enum": ["refuel", "dock", "undock"]},
                    "ship_id": {"type": "string", "description": "Optional ship UUID (defaults to lead ship)."},
                    "mode": {"type": "string", "enum": ["refined", "unrefined", "gas_giant_skim", "process_onboard"], "default": "refined"},
                    "tons": {"type": "number", "description": "Tons of fuel to add/process (omit to fill tanks)."}
                },
                "required": ["action"]
            }
        }),
        json!({
            "name": "personal_combat",
            "description": "Core 2022 6-second personal combat tracker: start combat, resolve attacks (ammo, range, cover, aim, reactions, armor-AP, END->DEX->STR damage & unconsciousness/death), run autonomous enemy AI counterattacks (`ai_turn`), advance round, or end combat.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "action": {"type": "string", "enum": ["start", "attack", "ai_turn", "next_round", "end"]},
                    "combatants": {"type": "array", "items": {"type": "object"}},
                    "attacker_id": {"type": "string"},
                    "defender_id": {"type": "string"},
                    "burst_mode": {"type": "boolean", "default": false},
                    "reaction": {"type": "string", "enum": ["none", "dodge", "parry", "dive_for_cover"], "default": "none"},
                    "defender_reaction_dm": {"type": "integer", "default": 0},
                    "extra_dm": {"type": "integer", "default": 0},
                    "auto_ai_turn": {"type": "boolean", "default": false}
                },
                "required": ["action"]
            }
        }),
        json!({
            "name": "space_combat",
            "description": "Core 2022 / High Guard 2024 6-minute spacecraft combat: start combat, allocate Thrust Pool (range shift, evasive action, aid gunners), fire linked turrets/barbettes (sandcasters, armor, 2D x Severity 1-6 Critical Hits on Effect 6+ and 10% hull loss), launch missiles, EW jam missiles, Sensor Lock (+2), run autonomous enemy fleet Tactical AI (`ai_turn`), advance round, or end combat.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "action": {"type": "string", "enum": ["start", "maneuver", "fire", "launch_missiles", "ew_jam", "sensor_lock", "ai_turn", "next_round", "end"]},
                    "range_band": {"type": "string", "enum": ["adjacent", "close", "short", "medium", "long", "very_long", "distant"], "default": "long"},
                    "ships": {"type": "array", "items": {"type": "object"}},
                    "ship_id": {"type": "string"},
                    "target_ship_id": {"type": "string"},
                    "thrust_to_range": {"type": "integer", "default": 0},
                    "thrust_to_evade": {"type": "integer", "default": 0},
                    "pilot_skill": {"type": "integer", "default": 1},
                    "aid_gunners": {"type": "boolean", "default": false},
                    "weapon_name": {"type": "string", "default": "Pulse Laser"},
                    "linked_count": {"type": "integer", "default": 1},
                    "damage_dice": {"type": "integer", "default": 2},
                    "is_laser": {"type": "boolean", "default": true},
                    "gunner_skill": {"type": "integer", "default": 1},
                    "gunner_dex_dm": {"type": "integer", "default": 0},
                    "fire_control_dm": {"type": "integer", "default": 0},
                    "sandcaster_count": {"type": "integer", "default": 0},
                    "missile_count": {"type": "integer", "default": 3},
                    "smart_tl": {"type": "integer", "default": 12},
                    "salvo_id": {"type": "string"},
                    "sensors_skill": {"type": "integer", "default": 1},
                    "edu_dm": {"type": "integer", "default": 0},
                    "auto_ai_turn": {"type": "boolean", "default": false}
                },
                "required": ["action"]
            }
        }),
        json!({
            "name": "advance_time",
            "description": "Advance the Imperial Calendar (DDD-YYYY HH:MM) by minutes/hours/days or execute a 1-week Jump (148 + 6D6 hours). Automatically completes due scheduled events AND when a 28-day Maintenance Period passes with pay_maintenance_periods=true, AUTOMATICALLY debits ship mortgages, 0.1%/yr maintenance, life support, and crew salaries from the Company account — NEVER call record_transaction again for those bills.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "minutes": {"type": "integer", "default": 0},
                    "hours": {"type": "integer", "default": 0},
                    "days": {"type": "integer", "default": 0},
                    "execute_jump": {"type": "boolean", "default": false},
                    "pay_maintenance_periods": {"type": "boolean", "default": true}
                }
            }
        }),
        json!({
            "name": "schedule_event",
            "description": "Schedule a future event/deadline/delivery on the Imperial Calendar (hours/days from now). Automatically triggers when advance_time crosses it.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "title": {"type": "string"},
                    "kind": {"type": "string", "default": "story"},
                    "hours_from_now": {"type": "integer", "default": 0},
                    "days_from_now": {"type": "integer", "default": 0}
                },
                "required": ["title"]
            }
        }),
        json!({
            "name": "record_transaction",
            "description": "Debit or credit a Personal or Company account and record a ledger transaction. Do NOT call this for 28-day Maintenance Periods (mortgage/maintenance/life support/salaries) that were already paid automatically by advance_time.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "account_id": {"type": "string"},
                    "amount_cr": {"type": "integer"},
                    "category": {"type": "string"},
                    "description": {"type": "string"},
                    "counterparty": {"type": "string", "default": ""}
                },
                "required": ["account_id", "amount_cr", "category", "description"]
            }
        }),
        json!({
            "name": "award_xp",
            "description": "Award Dedicated per-skill XP or General XP to any character (player, hired crew, or NPC).",
            "input_schema": {
                "type": "object",
                "properties": {
                    "character_id": {"type": "string"},
                    "skill": {"type": "string"},
                    "amount": {"type": "integer", "default": 1},
                    "reason": {"type": "string"}
                },
                "required": ["character_id", "amount"]
            }
        }),
        json!({
            "name": "validate_ship",
            "description": "Validate a ship's tonnage, power plant budget, jump fuel, hardpoints, and minimum crew against Mongoose Traveller 2e / High Guard 2024 rules.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "ship_id": {"type": "string"}
                },
                "required": ["ship_id"]
            }
        }),
        json!({
            "name": "update_ledger",
            "description": "Update one section of the campaign's narrative Story Ledger (who_knows_what, settled, people, or threads), shown after the cache breakpoint every turn.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "section": {"type": "string", "enum": ["who_knows_what", "settled", "people", "threads"]},
                    "text": {"type": "string"}
                },
                "required": ["section", "text"]
            }
        }),
        json!({
            "name": "supersede_last_reply",
            "description": "Withdraw the previous Referee reply after an out-of-character correction from the player: marks it superseded in history, deletes its Hindsight memories, and appends the correction to settled facts.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "correction": {"type": "string"}
                },
                "required": ["correction"]
            }
        }),
    ];

    if hindsight_enabled {
        specs.push(json!({
            "name": "recall_memory",
            "description": "Search this campaign's Hindsight long-term memory bank for earlier NPCs, promises, factions, clues, planets visited, or past scenes.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string"}
                },
                "required": ["query"]
            }
        }));
    }

    if let Some(last) = specs.last_mut() {
        last["cache_control"] = json!({"type": "ephemeral", "ttl": "1h"});
    }
    specs
}

pub struct ToolExecutionResult {
    pub output_json: String,
    pub state_changed: bool,
}

pub async fn execute_tool(
    db: &Db,
    hindsight: &HindsightClient,
    campaign_id: Uuid,
    name: &str,
    input: &Value,
) -> Result<ToolExecutionResult> {
    match name {
        "roll_task" => {
            let char_id = Uuid::parse_str(
                input
                    .get("character_id")
                    .and_then(|v| v.as_str())
                    .context("character_id required")?,
            )?;
            let mut row = db
                .get_character(campaign_id, char_id)
                .await?
                .context("Character not found")?;
            let char_str = input
                .get("characteristic")
                .and_then(|v| v.as_str())
                .unwrap_or("INT");
            let characteristic: Characteristic =
                serde_json::from_value(Value::String(char_str.to_uppercase()))
                    .unwrap_or(Characteristic::Int);
            let difficulty = Difficulty::from_str_loose(
                input
                    .get("difficulty")
                    .and_then(|v| v.as_str())
                    .unwrap_or("average"),
            )
            .unwrap_or(Difficulty::Average);
            let mode = match input.get("mode").and_then(|v| v.as_str()).unwrap_or("normal") {
                "boon" => RollMode::Boon,
                "bane" => RollMode::Bane,
                _ => RollMode::Normal,
            };
            let skill = input.get("skill").and_then(|v| v.as_str());
            let extra_dm = input.get("extra_dm").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let award_xp = input
                .get("award_activity_xp")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let mut mods = row.character.task_modifiers(skill, characteristic, difficulty);
            if extra_dm != 0 {
                mods.push(ModifierItem {
                    label: "Situational / Chain DM".to_string(),
                    value: extra_dm,
                });
            }

            let result = {
                let mut r = rng();
                resolve_task(&mut r, mode, difficulty, mods)
            };

            let mut xp_awarded = None;
            let mut state_changed = false;
            if award_xp && let Some(sk) = skill {
                let new_xp = row.character.award_skill_xp(sk, 1);
                db.save_character(&row.character, &row.equipment).await?;
                xp_awarded = Some(json!({"skill": sk, "dedicated_xp_total": new_xp}));
                state_changed = true;
            }

            Ok(ToolExecutionResult {
                output_json: serde_json::to_string(&json!({
                    "character": row.character.name,
                    "skill": skill,
                    "characteristic": characteristic,
                    "reason": input.get("reason").and_then(|v| v.as_str()).unwrap_or(""),
                    "result": result,
                    "xp_awarded": xp_awarded
                }))?,
                state_changed,
            })
        }

        "roll_dice" => {
            let kind = input.get("kind").and_then(|v| v.as_str()).unwrap_or("nd6");
            let dm = input.get("dm").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let out = {
                let mut r = rng();
                if kind == "d66" {
                    let d66 = roll_d66(&mut r);
                    json!({"kind": "d66", "result": d66, "reason": input.get("reason")})
                } else {
                    let count =
                        input.get("count").and_then(|v| v.as_u64()).unwrap_or(2).clamp(1, 20) as usize;
                    let dice = roll_nd6(&mut r, count);
                    let sum: i32 = dice.iter().map(|&d| i32::from(d)).sum();
                    json!({"kind": format!("{count}D6"), "dice": dice, "dm": dm, "total": sum + dm, "reason": input.get("reason")})
                }
            };
            Ok(ToolExecutionResult {
                output_json: serde_json::to_string(&out)?,
                state_changed: false,
            })
        }

        "sensor_check" => {
            let context = match input
                .get("context")
                .and_then(|v| v.as_str())
                .unwrap_or("space_initial_detection")
            {
                "space_stealthed_reacquire" => SensorContext::SpaceStealthedReacquire,
                "personal_ground" => SensorContext::PersonalGround,
                _ => SensorContext::SpaceInitialDetection,
            };
            let grade = match input
                .get("observer_sensor_grade")
                .and_then(|v| v.as_str())
                .unwrap_or("civilian")
            {
                "basic" => SensorGrade::Basic,
                "military" => SensorGrade::Military,
                "improved" => SensorGrade::Improved,
                "advanced" => SensorGrade::Advanced,
                _ => SensorGrade::Civilian,
            };

            let (mut edu_dm, mut skill_lvl, mut aug_dm) = (0, 0, 0);
            if let Some(cid_str) = input.get("operator_character_id").and_then(|v| v.as_str())
                && let Ok(cid) = Uuid::parse_str(cid_str)
                && let Some(row) = db.get_character(campaign_id, cid).await?
            {
                let mods = row.character.task_modifiers(
                    Some("Electronics (sensors)"),
                    Characteristic::Edu,
                    Difficulty::Average,
                );
                edu_dm = mods.first().map(|m| m.value).unwrap_or(0);
                skill_lvl = mods.get(1).map(|m| m.value).unwrap_or(-3);
                aug_dm = mods.iter().skip(2).map(|m| m.value).sum();
            }

            let req = SensorDetectionInput {
                context,
                operator_edu_dm: edu_dm,
                operator_sensors_skill: skill_lvl,
                augment_or_software_dm: aug_dm,
                observer_sensor_grade: grade,
                observer_tl: input.get("observer_tl").and_then(|v| v.as_u64()).unwrap_or(12) as u8,
                observer_extended_array: input
                    .get("observer_extended_array")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                observer_active_sensors: input
                    .get("observer_active_sensors")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
                target_tl: input.get("target_tl").and_then(|v| v.as_u64()).unwrap_or(12) as u8,
                target_active_sensors: input
                    .get("target_active_sensors")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                target_thrust_applied: input
                    .get("target_thrust_applied")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u8,
                target_power_plant_active: input
                    .get("target_power_plant_active")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
                target_transponder_or_comms: input
                    .get("target_transponder_or_comms")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                target_stealth_armor_dm: input
                    .get("target_stealth_armor_dm")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32,
                target_fired_weapons: input
                    .get("target_fired_weapons")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                target_crit_damage_severity: input
                    .get("target_crit_damage_severity")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32,
                situational_dm: input
                    .get("situational_dm")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32,
                roll_mode: RollMode::Normal,
            };

            let res = {
                let mut r = rng();
                resolve_sensor_detection(&mut r, &req)
            };
            Ok(ToolExecutionResult {
                output_json: serde_json::to_string(&res)?,
                state_changed: false,
            })
        }

        "explore_universe" => {
            let action = input
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("search");
            match action {
                "search" => {
                    let systems = db
                        .search_systems(
                            input.get("query").and_then(|v| v.as_str()),
                            input.get("trade_code").and_then(|v| v.as_str()),
                            25,
                        )
                        .await?;
                    Ok(ToolExecutionResult {
                        output_json: serde_json::to_string(&json!({ "systems": systems }))?,
                        state_changed: false,
                    })
                }
                "route" => {
                    let camp = db
                        .get_campaign(campaign_id)
                        .await?
                        .context("Campaign not found")?;
                    let from_name = input
                        .get("from_system")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&camp.current_system_name);
                    let to_name = input
                        .get("to_system")
                        .and_then(|v| v.as_str())
                        .context("to_system required")?;
                    let jump_rating =
                        input.get("jump_rating").and_then(|v| v.as_u64()).unwrap_or(1) as u8;

                    let from = db
                        .get_system_by_name(from_name)
                        .await?
                        .with_context(|| format!("Origin system '{from_name}' not found"))?;
                    let to = db
                        .get_system_by_name(to_name)
                        .await?
                        .with_context(|| format!("Destination system '{to_name}' not found"))?;

                    let pad = 8;
                    let box_sys = db
                        .list_systems_in_box(
                            from.global_x.min(to.global_x) - pad,
                            from.global_x.max(to.global_x) + pad,
                            from.global_y.min(to.global_y) - pad,
                            from.global_y.max(to.global_y) + pad,
                        )
                        .await?;
                    let nodes: Vec<RouteNode> = box_sys
                        .into_iter()
                        .map(|s| RouteNode {
                            id: s.id,
                            name: s.name,
                            hex: s.hex,
                            uwp: s.uwp,
                            starport: s.starport.chars().next().unwrap_or('X'),
                            gas_giants: s.gas_giants,
                            travel_zone: s.travel_zone,
                            global_x: s.global_x,
                            global_y: s.global_y,
                        })
                        .collect();
                    let route = plan_jump_route(&nodes, from.id, to.id, jump_rating, true, true);
                    Ok(ToolExecutionResult {
                        output_json: serde_json::to_string(&json!({ "route": route }))?,
                        state_changed: false,
                    })
                }
                "jump_to" => {
                    let camp = db
                        .get_campaign(campaign_id)
                        .await?
                        .context("Campaign not found")?;
                    let to_name = input
                        .get("to_system")
                        .and_then(|v| v.as_str())
                        .context("to_system required")?;
                    let dest = db
                        .get_system_by_name(to_name)
                        .await?
                        .with_context(|| format!("Destination system '{to_name}' not found"))?;

                    // Compute exact parsec distance from current system
                    let parsecs: u8 = if let Some(origin) = db.get_system_by_name(&camp.current_system_name).await? {
                        hex_distance(origin.global_x, origin.global_y, dest.global_x, dest.global_y)
                            .clamp(1, 20) as u8
                    } else {
                        1
                    };

                    // Enforce FLEET jump constraint: fleet can only jump the distance of the LOWEST Jump Rating ship!
                    let mut ships = db.list_ships(campaign_id).await?;
                    let mut fuel_burned_total = 0.0;
                    let mut fleet_fuel_log = Vec::new();
                    if !ships.is_empty() {
                        let specs: Vec<_> = ships.iter().map(|s| &s.spec).collect();
                        let fleet_max_j = fleet_max_jump_rating(&specs);
                        if parsecs > fleet_max_j {
                            let bottleneck = ships
                                .iter()
                                .min_by_key(|s| s.spec.jump_rating)
                                .map(|s| format!("{} (J-{})", s.name, s.spec.jump_rating))
                                .unwrap_or_else(|| format!("J-{fleet_max_j}"));
                            anyhow::bail!(
                                "Fleet cannot jump {parsecs} parsecs to {}: maximum fleet jump range is Jump-{fleet_max_j} (limited by lowest-jump ship {bottleneck}). Plot a multi-leg route instead!",
                                dest.name
                            );
                        }

                        // Pre-validate fuel across EVERY ship in the fleet before deducting
                        for ship in &ships {
                            let req_fuel = jump_fuel_for_distance(ship.spec.tonnage, parsecs);
                            let total_fuel = ship.fuel_refined + ship.fuel_unrefined;
                            if total_fuel + 0.01 < req_fuel {
                                anyhow::bail!(
                                    "Fleet ship {} needs {:.1}t fuel for a {parsecs}-parsec jump, only has {:.1}t! Refuel before jumping.",
                                    ship.name,
                                    req_fuel,
                                    total_fuel
                                );
                            }
                        }

                        // Deduct fuel & move EVERY ship in the fleet together
                        for ship in &mut ships {
                            let req_fuel = jump_fuel_for_distance(ship.spec.tonnage, parsecs);
                            let from_ref = ship.fuel_refined.min(req_fuel);
                            ship.fuel_refined -= from_ref;
                            ship.fuel_unrefined = (ship.fuel_unrefined - (req_fuel - from_ref)).max(0.0);
                            ship.docked_system_name = dest.name.clone();
                            ship.is_docked = false;
                            ship.docked_at_total_minutes = None;
                            ship.berthing_billed_days = 0;
                            db.save_ship(ship).await?;
                            fuel_burned_total += req_fuel;
                            fleet_fuel_log.push(json!({
                                "ship": ship.name,
                                "jump_rating": ship.spec.jump_rating,
                                "fuel_burned_tons": req_fuel,
                                "fuel_remaining_tons": ship.fuel_refined + ship.fuel_unrefined
                            }));
                        }
                    }

                    db.update_campaign_system(campaign_id, &dest.name, &dest.uwp, Some(dest.id))
                        .await?;
                    let time_res = Box::pin(execute_tool(
                        db,
                        hindsight,
                        campaign_id,
                        "advance_time",
                        &json!({"execute_jump": true, "pay_maintenance_periods": true}),
                    ))
                    .await?;
                    let time_json: Value = serde_json::from_str(&time_res.output_json)?;

                    // Automatically sweep & persist in-system starship traffic on arrival!
                    let dest_uwp = parse_uwp(&dest.uwp)
                        .unwrap_or_else(|_| parse_uwp("C555555-9").expect("valid fallback uwp"));
                    let traffic_report = {
                        let mut r = rng();
                        generate_system_traffic(
                            &mut r,
                            &dest.name,
                            &dest_uwp,
                            &dest.bases,
                            &dest.travel_zone,
                        )
                    };
                    let stored_ships = db
                        .replace_system_ships(campaign_id, &dest.name, &traffic_report.vessels)
                        .await?;

                    Ok(ToolExecutionResult {
                        output_json: serde_json::to_string(&json!({
                            "arrived_at": dest.name,
                            "uwp": dest.uwp,
                            "parsecs_jumped": parsecs,
                            "arrived_system": dest,
                            "fuel_burned_tons": fuel_burned_total,
                            "fleet_fuel_log": fleet_fuel_log,
                            "time_advance": time_json,
                            "in_system_ship_count": stored_ships.len(),
                            "traffic_summary": traffic_report.summary,
                            "in_system_ships": stored_ships
                        }))?,
                        state_changed: true,
                    })
                }
                other => anyhow::bail!("Unknown explore_universe action '{other}'"),
            }
        }

        "trade_ops" => {
            let camp = db
                .get_campaign(campaign_id)
                .await?
                .context("Campaign not found")?;
            let action = input
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("market");
            let origin_uwp = parse_uwp(&camp.current_system_uwp)?;

            match action {
                "market" => {
                    let chars = db.list_characters(campaign_id).await?;
                    let ships = db.list_ships(campaign_id).await?;
                    let roster_refs: Vec<(&traveller_rules::character::Character, &[Value])> = chars
                        .iter()
                        .map(|r| (&r.character, r.equipment.as_slice()))
                        .collect();
                    let fleet_assets: Vec<Value> = ships
                        .iter()
                        .flat_map(|s| s.cargo.iter().cloned())
                        .collect();
                    let fleet_broker = compute_fleet_max_broker(&roster_refs, &fleet_assets);

                    let dest_name = input
                        .get("dest_system")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Roup");
                    let dest_uwp = if let Some(s) = db.get_system_by_name(dest_name).await? {
                        parse_uwp(&s.uwp).unwrap_or_else(|_| origin_uwp.clone())
                    } else {
                        origin_uwp.clone()
                    };
                    let parsecs = input.get("parsecs").and_then(|v| v.as_u64()).unwrap_or(1) as u8;
                    // Automatically use MAX Broker & Steward across the entire fleet (including robots & Neural Jacks!)
                    let broker = input
                        .get("broker_skill")
                        .and_then(|v| v.as_i64())
                        .map(|v| (v as i32).max(fleet_broker.max_broker_skill))
                        .unwrap_or(fleet_broker.max_broker_skill);
                    let steward = input
                        .get("steward_skill")
                        .and_then(|v| v.as_i64())
                        .map(|v| (v as i32).max(fleet_broker.max_steward_skill))
                        .unwrap_or(fleet_broker.max_steward_skill);
                    let illegal = input.get("include_illegal").and_then(|v| v.as_bool()).unwrap_or(false);

                    let market = {
                        let mut r = rng();
                        generate_world_market(
                            &mut r,
                            &camp.current_system_name,
                            &origin_uwp,
                            "Green",
                            dest_name,
                            &dest_uwp,
                            parsecs,
                            broker,
                            steward,
                            illegal,
                        )
                    };
                    let mut market_val = serde_json::to_value(&market)?;
                    if let Some(obj) = market_val.as_object_mut() {
                        obj.insert("effective_broker_skill".into(), json!(broker));
                        obj.insert("effective_steward_skill".into(), json!(steward));
                        obj.insert("fleet_broker".into(), serde_json::to_value(&fleet_broker)?);
                    }
                    Ok(ToolExecutionResult {
                        output_json: serde_json::to_string(&market_val)?,
                        state_changed: false,
                    })
                }
                "buy_cargo" => {
                    let d66 = input.get("d66").and_then(|v| v.as_u64()).context("d66 required")? as u8;
                    let good_name = input
                        .get("good_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Speculative Cargo");
                    let tons = input.get("tons").and_then(|v| v.as_u64()).context("tons required")? as u32;
                    let unit_price = input
                        .get("unit_price_cr")
                        .and_then(|v| v.as_i64())
                        .context("unit_price_cr required")?;
                    let total_cost = i64::from(tons) * unit_price;

                    let mut ships = db.list_ships(campaign_id).await?;
                    let ship = ships.first_mut().context("No ship owned")?;
                    let used_tons: f64 = ship
                        .cargo
                        .iter()
                        .filter_map(|c| c.get("tons").and_then(|v| v.as_f64()))
                        .sum();
                    if used_tons + f64::from(tons) > ship.spec.cargo_capacity_tons + 0.01 {
                        anyhow::bail!(
                            "Insufficient cargo hold: {:.1}t used + {}t > {:.1}t cap",
                            used_tons,
                            tons,
                            ship.spec.cargo_capacity_tons
                        );
                    }

                    let accounts = db.list_accounts(campaign_id).await?;
                    let company = accounts
                        .iter()
                        .find(|a| a.kind == AccountKind::Company)
                        .context("Company account not found")?;
                    let tx = db
                        .post_transaction(
                            campaign_id,
                            company.id,
                            &camp.date.format_date_only(),
                            -total_cost,
                            "trade_buy",
                            &format!("Bought {tons}t {good_name} @ Cr{unit_price}/t on {}", camp.current_system_name),
                            &camp.current_system_name,
                        )
                        .await?;
                    ship.cargo.push(json!({
                        "d66": d66,
                        "name": good_name,
                        "tons": tons,
                        "unit_cost_cr": unit_price,
                        "origin": camp.current_system_name
                    }));
                    db.save_ship(ship).await?;
                    Ok(ToolExecutionResult {
                        output_json: serde_json::to_string(&json!({
                            "purchased": {"d66": d66, "name": good_name, "tons": tons, "total_cost_cr": total_cost},
                            "balance_after_cr": tx.balance_after_cr
                        }))?,
                        state_changed: true,
                    })
                }
                "sell_cargo" => {
                    let chars = db.list_characters(campaign_id).await?;
                    let mut ships = db.list_ships(campaign_id).await?;
                    let roster_refs: Vec<(&traveller_rules::character::Character, &[Value])> = chars
                        .iter()
                        .map(|r| (&r.character, r.equipment.as_slice()))
                        .collect();
                    let fleet_assets: Vec<Value> = ships
                        .iter()
                        .flat_map(|s| s.cargo.iter().cloned())
                        .collect();
                    let fleet_broker = compute_fleet_max_broker(&roster_refs, &fleet_assets);

                    let d66 = input.get("d66").and_then(|v| v.as_u64()).context("d66 required")? as u8;
                    let broker = input
                        .get("broker_skill")
                        .and_then(|v| v.as_i64())
                        .map(|v| (v as i32).max(fleet_broker.max_broker_skill))
                        .unwrap_or(fleet_broker.max_broker_skill);
                    let ship = ships.first_mut().context("No ship owned")?;
                    let idx = ship
                        .cargo
                        .iter()
                        .position(|c| c.get("d66").and_then(|v| v.as_u64()) == Some(u64::from(d66)))
                        .context("No matching cargo lot in ship hold")?;
                    let lot = ship.cargo.remove(idx);
                    let tons = lot.get("tons").and_then(|v| v.as_i64()).unwrap_or(1);
                    let cost_basis = lot.get("unit_cost_cr").and_then(|v| v.as_i64()).unwrap_or(0) * tons;
                    let lot_name = lot
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Cargo")
                        .to_string();

                    let (sale_pct, unit_sale, roll_total) = {
                        let mut r = rng();
                        roll_speculative_sale(&mut r, d66, &origin_uwp, "Green", broker, 2)
                            .context("Invalid trade good d66")?
                    };
                    let proceeds = unit_sale * tons;
                    let profit = proceeds - cost_basis;

                    let accounts = db.list_accounts(campaign_id).await?;
                    let company = accounts
                        .iter()
                        .find(|a| a.kind == AccountKind::Company)
                        .context("Company account not found")?;
                    let tx = db
                        .post_transaction(
                            campaign_id,
                            company.id,
                            &camp.date.format_date_only(),
                            proceeds,
                            "trade_sell",
                            &format!("Sold {tons}t {lot_name} @ Cr{unit_sale}/t ({sale_pct}%, profit Cr{profit:+})"),
                            &camp.current_system_name,
                        )
                        .await?;
                    db.save_ship(ship).await?;
                    Ok(ToolExecutionResult {
                        output_json: serde_json::to_string(&json!({
                            "sold": lot_name,
                            "tons": tons,
                            "sale_percent": sale_pct,
                            "price_roll": roll_total,
                            "unit_price_cr": unit_sale,
                            "proceeds_cr": proceeds,
                            "profit_cr": profit,
                            "balance_after_cr": tx.balance_after_cr
                        }))?,
                        state_changed: true,
                    })
                }
                other => anyhow::bail!("Unknown trade_ops action '{other}'"),
            }
        }

        "shop_market" => {
            let camp = db
                .get_campaign(campaign_id)
                .await?
                .context("Campaign not found")?;
            let action = input
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("search");
            match action {
                "search" => {
                    let items = db
                        .search_items(
                            input.get("query").and_then(|v| v.as_str()),
                            input.get("category").and_then(|v| v.as_str()),
                            input.get("max_tl").and_then(|v| v.as_i64()).map(|x| x as i16),
                            25,
                        )
                        .await?;
                    Ok(ToolExecutionResult {
                        output_json: serde_json::to_string(&json!({ "items": items }))?,
                        state_changed: false,
                    })
                }
                "check_availability" => {
                    let item_id = Uuid::parse_str(
                        input
                            .get("item_id")
                            .and_then(|v| v.as_str())
                            .context("item_id required")?,
                    )?;
                    let item = db.get_item(item_id).await?.context("Item not found")?;
                    let world = parse_uwp(&camp.current_system_uwp)?;
                    let cat = match item.law_restriction {
                        0..=1 => LegalityCategory::Category5RestrictedMilitary,
                        2..=3 => LegalityCategory::Category4Military,
                        4..=5 => LegalityCategory::Category3Paramilitary,
                        6..=7 => LegalityCategory::Category2Civilian,
                        _ => LegalityCategory::Category1Unrestricted,
                    };
                    let req = AvailabilityInput {
                        item_name: item.name.clone(),
                        item_tl: item.tl as u8,
                        base_cost_cr: item.cost_cr,
                        specialized: item.category == "augment" || item.category == "software",
                        military_use: item.law_restriction <= 3,
                        category: cat,
                        black_market: input
                            .get("black_market")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        willing_price_multiplier: input
                            .get("willing_price_multiplier")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(1) as u8,
                        previous_attempts_this_month: 0,
                        skill_dm: input.get("skill_dm").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                        stat_dm: input.get("stat_dm").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                        referee_dm: 0,
                    };
                    let res = {
                        let mut r = rng();
                        check_item_availability(&mut r, &world, &req)
                    };
                    Ok(ToolExecutionResult {
                        output_json: serde_json::to_string(&res)?,
                        state_changed: false,
                    })
                }
                "purchase" => {
                    let item_id = Uuid::parse_str(
                        input
                            .get("item_id")
                            .and_then(|v| v.as_str())
                            .context("item_id required")?,
                    )?;
                    let item = db.get_item(item_id).await?.context("Item not found")?;
                    let qty = input.get("quantity").and_then(|v| v.as_i64()).unwrap_or(1).max(1);
                    let unit_price = input
                        .get("unit_price_cr")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(item.cost_cr);
                    let total = unit_price * qty;

                    let accounts = db.list_accounts(campaign_id).await?;
                    let acc_id = if let Some(s) = input.get("account_id").and_then(|v| v.as_str()) {
                        Uuid::parse_str(s)?
                    } else {
                        accounts
                            .iter()
                            .find(|a| a.kind == AccountKind::Personal)
                            .or_else(|| accounts.first())
                            .context("No account found")?
                            .id
                    };
                    let tx = db
                        .post_transaction(
                            campaign_id,
                            acc_id,
                            &camp.date.format_date_only(),
                            -total,
                            "purchase",
                            &format!("Purchased {qty}x {} @ Cr{}", item.name, unit_price),
                            &camp.current_system_name,
                        )
                        .await?;
                    if let Some(cid_s) = input.get("character_id").and_then(|v| v.as_str())
                        && let Ok(cid) = Uuid::parse_str(cid_s)
                        && let Some(mut row) = db.get_character(campaign_id, cid).await?
                    {
                        row.equipment.push(json!({
                            "item_id": item.id,
                            "name": item.name,
                            "category": item.category,
                            "tl": item.tl,
                            "quantity": qty,
                            "mass_kg": item.mass_kg,
                            "stats": item.stats,
                            "equipped": true
                        }));
                        db.save_character(&row.character, &row.equipment).await?;
                    }
                    Ok(ToolExecutionResult {
                        output_json: serde_json::to_string(&json!({
                            "item": item.name,
                            "quantity": qty,
                            "total_cr": total,
                            "balance_after_cr": tx.balance_after_cr
                        }))?,
                        state_changed: true,
                    })
                }
                other => anyhow::bail!("Unknown shop_market action '{other}'"),
            }
        }

        "hire_crew" => {
            let camp = db
                .get_campaign(campaign_id)
                .await?
                .context("Campaign not found")?;
            let action = input
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("hiring_hall");
            let world = parse_uwp(&camp.current_system_uwp)?;
            match action {
                "hiring_hall" => {
                    let roles: Vec<String> = input
                        .get("roles")
                        .and_then(|v| v.as_array())
                        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                        .unwrap_or_default();
                    let pool = {
                        let mut r = rng();
                        generate_hiring_hall(&mut r, campaign_id, &world, &roles)
                    };
                    Ok(ToolExecutionResult {
                        output_json: serde_json::to_string(&json!({ "candidates": pool }))?,
                        state_changed: false,
                    })
                }
                "hire" => {
                    let role = input.get("role").and_then(|v| v.as_str()).unwrap_or("Engineer");
                    let roles = vec![role.to_string()];
                    let mut cand = {
                        let mut r = rng();
                        generate_hiring_hall(&mut r, campaign_id, &world, &roles)
                            .into_iter()
                            .next()
                            .context("Failed to generate crew candidate")?
                    };
                    if let Some(n) = input.get("name").and_then(|v| v.as_str()) {
                        cand.character.name = n.to_string();
                    }
                    if let Some(sal) = input.get("salary_cr").and_then(|v| v.as_i64()) {
                        cand.character.salary_cr = sal;
                    }
                    let ships = db.list_ships(campaign_id).await?;
                    let ship_id = input
                        .get("ship_id")
                        .and_then(|v| v.as_str())
                        .and_then(|s| Uuid::parse_str(s).ok())
                        .or_else(|| ships.first().map(|s| s.id));

                    let created = db
                        .create_character(
                            campaign_id,
                            traveller_db::NewCharacterInput {
                                kind: CharacterKind::Crew,
                                name: cand.character.name,
                                species: cand.character.species,
                                age: cand.character.age,
                                upp: cand.character.upp_max,
                                skills: cand.character.skills,
                                augments: vec![],
                                salary_cr: cand.character.salary_cr,
                                assigned_ship_id: ship_id,
                                assigned_station: Some(role.to_string()),
                                notes: cand.character.notes,
                            },
                        )
                        .await?;
                    Ok(ToolExecutionResult {
                        output_json: serde_json::to_string(&json!({ "hired": created }))?,
                        state_changed: true,
                    })
                }
                other => anyhow::bail!("Unknown hire_crew action '{other}'"),
            }
        }

        "personal_combat" => {
            let action = input.get("action").and_then(|v| v.as_str()).unwrap_or("start");
            match action {
                "start" => {
                    let mut combatants: Vec<PersonalCombatant> =
                        if let Some(arr) = input.get("combatants").and_then(|v| v.as_array()) {
                            arr.iter()
                                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                                .collect()
                        } else {
                            vec![]
                        };
                    if combatants.is_empty() {
                        // Auto-seed party characters from DB
                        for row in db.list_characters(campaign_id).await? {
                            let c = row.character;
                            combatants.push(PersonalCombatant {
                                character_id: c.id,
                                name: c.name,
                                side: if c.kind == CharacterKind::Npc { "hostile".into() } else { "party".into() },
                                initiative: 0,
                                next_round_init_penalty: 0,
                                armor: 4,
                                cover_dm: 0,
                                aim_bonus: 0,
                                range_band: PersonalRangeBand::Medium,
                                weapon: PersonalWeapon {
                                    name: "Autopistol".into(),
                                    skill: "Gun Combat (Slug)".into(),
                                    range_m: 10,
                                    damage_dice: 3,
                                    damage_bonus: -3,
                                    ap: 0,
                                    auto_rating: 0,
                                    stun: false,
                                    magazine_current: 15,
                                    magazine_max: 15,
                                },
                                conscious: true,
                                alive: true,
                                str_cur: c.upp_current.str_val,
                                dex_cur: c.upp_current.dex_val,
                                end_cur: c.upp_current.end_val,
                                action_used_this_round: false,
                            });
                        }
                    }
                    let state = {
                        let mut r = rng();
                        start_personal_combat(&mut r, combatants, 0, 0)
                    };
                    let val = serde_json::to_value(&state)?;
                    db.upsert_combat(state.id, campaign_id, "personal", "active", 1, "active", &val)
                        .await?;
                    Ok(ToolExecutionResult {
                        output_json: serde_json::to_string(&state)?,
                        state_changed: true,
                    })
                }
                "attack" => {
                    let rec = db
                        .get_active_combat(campaign_id, "personal")
                        .await?
                        .context("No active personal combat")?;
                    let mut state: PersonalCombatState = serde_json::from_value(rec.state)?;
                    let atk_id = Uuid::parse_str(
                        input.get("attacker_id").and_then(|v| v.as_str()).context("attacker_id required")?,
                    )?;
                    let def_id = Uuid::parse_str(
                        input.get("defender_id").and_then(|v| v.as_str()).context("defender_id required")?,
                    )?;
                    let reaction = match input.get("reaction").and_then(|v| v.as_str()).unwrap_or("none") {
                        "dodge" => ReactionKind::Dodge,
                        "parry" => ReactionKind::Parry,
                        "dive_for_cover" => ReactionKind::DiveForCover,
                        _ => ReactionKind::None,
                    };
                    let atk_char = db
                        .get_character(campaign_id, atk_id)
                        .await?
                        .map(|r| r.character);
                    let res = {
                        let mut r = rng();
                        resolve_personal_attack(
                            &mut r,
                            &mut state,
                            atk_char.as_ref(),
                            &PersonalAttackInput {
                                attacker_id: atk_id,
                                defender_id: def_id,
                                burst_mode: input.get("burst_mode").and_then(|v| v.as_bool()).unwrap_or(false),
                                reaction,
                                defender_reaction_dm: input.get("defender_reaction_dm").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                                extra_dm: input.get("extra_dm").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                            },
                        )?
                    };
                    // Sync defender wounds back to characters table if they're a tracked character
                    if let Some(def_c) = state.combatants.iter().find(|c| c.character_id == def_id)
                        && let Some(mut row) = db.get_character(campaign_id, def_id).await?
                    {
                        row.character.upp_current.end_val = def_c.end_cur;
                        row.character.upp_current.dex_val = def_c.dex_cur;
                        row.character.upp_current.str_val = def_c.str_cur;
                        db.save_character(&row.character, &row.equipment).await?;
                    }
                    let val = serde_json::to_value(&state)?;
                    db.upsert_combat(rec.id, campaign_id, "personal", "active", state.round as i32, "active", &val)
                        .await?;
                    Ok(ToolExecutionResult {
                        output_json: serde_json::to_string(&res)?,
                        state_changed: true,
                    })
                }
                "ai_turn" => {
                    let rec = db
                        .get_active_combat(campaign_id, "personal")
                        .await?
                        .context("No active personal combat")?;
                    let mut state: PersonalCombatState = serde_json::from_value(rec.state)?;
                    let reaction = match input.get("reaction").and_then(|v| v.as_str()).unwrap_or("none") {
                        "dodge" => ReactionKind::Dodge,
                        "parry" => ReactionKind::Parry,
                        "dive_for_cover" => ReactionKind::DiveForCover,
                        _ => ReactionKind::None,
                    };
                    let react_dm = input
                        .get("defender_reaction_dm")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(1) as i32;
                    let report = {
                        let mut r = rng();
                        execute_personal_ai_turn(&mut r, &mut state, reaction, react_dm)
                    };
                    // Sync any wounded party characters back to PostgreSQL
                    for c in &state.combatants {
                        if c.side == "party"
                            && let Some(mut row) = db.get_character(campaign_id, c.character_id).await?
                        {
                            row.character.upp_current.end_val = c.end_cur;
                            row.character.upp_current.dex_val = c.dex_cur;
                            row.character.upp_current.str_val = c.str_cur;
                            db.save_character(&row.character, &row.equipment).await?;
                        }
                    }
                    let val = serde_json::to_value(&state)?;
                    db.upsert_combat(rec.id, campaign_id, "personal", "active", state.round as i32, "active", &val)
                        .await?;
                    Ok(ToolExecutionResult {
                        output_json: serde_json::to_string(&report)?,
                        state_changed: true,
                    })
                }
                "next_round" => {
                    let rec = db
                        .get_active_combat(campaign_id, "personal")
                        .await?
                        .context("No active personal combat")?;
                    let mut state: PersonalCombatState = serde_json::from_value(rec.state)?;
                    let auto_ai = input.get("auto_ai_turn").and_then(|v| v.as_bool()).unwrap_or(false);
                    let ai_report = if auto_ai {
                        let rep = {
                            let mut r = rng();
                            execute_personal_ai_turn(&mut r, &mut state, ReactionKind::None, 0)
                        };
                        for c in &state.combatants {
                            if c.side == "party"
                                && let Some(mut row) = db.get_character(campaign_id, c.character_id).await?
                            {
                                row.character.upp_current.end_val = c.end_cur;
                                row.character.upp_current.dex_val = c.dex_cur;
                                row.character.upp_current.str_val = c.str_cur;
                                db.save_character(&row.character, &row.equipment).await?;
                            }
                        }
                        Some(rep)
                    } else {
                        None
                    };
                    next_personal_round(&mut state);
                    let val = serde_json::to_value(&state)?;
                    db.upsert_combat(rec.id, campaign_id, "personal", "active", state.round as i32, "active", &val)
                        .await?;
                    Ok(ToolExecutionResult {
                        output_json: json!({"round": state.round, "ai_report": ai_report, "state": state}).to_string(),
                        state_changed: true,
                    })
                }
                "end" => {
                    if let Some(rec) = db.get_active_combat(campaign_id, "personal").await? {
                        db.upsert_combat(rec.id, campaign_id, "personal", "ended", rec.round, "ended", &rec.state)
                            .await?;
                    }
                    Ok(ToolExecutionResult {
                        output_json: json!({"status": "ended"}).to_string(),
                        state_changed: true,
                    })
                }
                other => anyhow::bail!("Unknown personal_combat action '{other}'"),
            }
        }

        "space_combat" => {
            let action = input.get("action").and_then(|v| v.as_str()).unwrap_or("start");
            match action {
                "start" => {
                    let range_band: SpaceRangeBand = serde_json::from_value(
                        input.get("range_band").cloned().unwrap_or_else(|| json!("long")),
                    )
                    .unwrap_or(SpaceRangeBand::Long);
                    let mut ships_list: Vec<SpaceCombatShip> =
                        if let Some(arr) = input.get("ships").and_then(|v| v.as_array()) {
                            arr.iter()
                                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                                .collect()
                        } else {
                            vec![]
                        };
                    if ships_list.is_empty() {
                        let camp = db
                            .get_campaign(campaign_id)
                            .await?
                            .context("Campaign not found")?;
                        let in_system = db
                            .list_system_ships(campaign_id, &camp.current_system_name)
                            .await?;
                        let target_vessel = in_system
                            .iter()
                            .find(|v| v.status != "destroyed" && v.disposition == "hostile")
                            .or_else(|| in_system.iter().find(|v| v.status != "destroyed"))
                            .cloned()
                            .context(
                                "No target ship specified and no vessels are currently in-system to engage. Select an in-system contact or pass explicit ships.",
                            )?;

                        for s in db.list_ships(campaign_id).await? {
                            ships_list.push(SpaceCombatShip {
                                ship_id: s.id,
                                name: s.name,
                                side: "party".into(),
                                tonnage: s.spec.tonnage,
                                tl: s.spec.tl,
                                hull_max: s.spec.hull_points_max,
                                hull_current: s.hull_current,
                                armor: s.spec.armor_rating,
                                thrust_max: s.spec.thrust,
                                thrust_remaining: s.spec.thrust,
                                evasive_dm: 0,
                                aid_gunners_dm: 0,
                                power_available: s.spec.power_plant_output,
                                sensor_lock_targets: vec![],
                                crits: Default::default(),
                                initiative: 0,
                                last_ten_pct_threshold: 0,
                                maneuver_used: false,
                                weapons_fired_this_round: 0,
                                max_weapons_per_round: (s.spec.tonnage / 200).max(1),
                                sensor_action_used: false,
                                missiles_launched_this_round: false,
                            });
                        }
                        ships_list.push(SpaceCombatShip {
                            ship_id: target_vessel.id,
                            name: target_vessel.name,
                            side: "hostile".into(),
                            tonnage: target_vessel.tonnage.max(0) as u32,
                            tl: target_vessel.tl.max(0) as u8,
                            hull_max: target_vessel.hull_max,
                            hull_current: target_vessel.hull_current,
                            armor: target_vessel.armor,
                            thrust_max: target_vessel.thrust.max(0) as u8,
                            thrust_remaining: target_vessel.thrust.max(0) as u8,
                            evasive_dm: 0,
                            aid_gunners_dm: 0,
                            power_available: (target_vessel.tonnage / 2).max(60),
                            sensor_lock_targets: vec![],
                            crits: Default::default(),
                            initiative: 0,
                            last_ten_pct_threshold: 0,
                            maneuver_used: false,
                            weapons_fired_this_round: 0,
                            max_weapons_per_round: ((target_vessel.tonnage.max(0) as u32) / 200).max(1),
                            sensor_action_used: false,
                            missiles_launched_this_round: false,
                        });
                    }
                    let state = {
                        let mut r = rng();
                        start_space_combat(&mut r, range_band, ships_list, 0, 0)
                    };
                    let val = serde_json::to_value(&state)?;
                    db.upsert_combat(state.id, campaign_id, "space", "active", 1, "maneuver", &val)
                        .await?;
                    Ok(ToolExecutionResult {
                        output_json: serde_json::to_string(&state)?,
                        state_changed: true,
                    })
                }
                "maneuver" => {
                    let rec = db.get_active_combat(campaign_id, "space").await?.context("No active space combat")?;
                    let mut state: SpaceCombatState = serde_json::from_value(rec.state)?;
                    let ship_id = Uuid::parse_str(input.get("ship_id").and_then(|v| v.as_str()).context("ship_id required")?)?;
                    let msg = {
                        let mut r = rng();
                        allocate_maneuver_thrust(
                            &mut r,
                            &mut state,
                            ship_id,
                            input.get("thrust_to_range").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                            input.get("thrust_to_evade").and_then(|v| v.as_u64()).unwrap_or(0) as u8,
                            input.get("pilot_skill").and_then(|v| v.as_i64()).unwrap_or(1) as i32,
                            input.get("aid_gunners").and_then(|v| v.as_bool()).unwrap_or(false),
                        )?
                    };
                    let val = serde_json::to_value(&state)?;
                    db.upsert_combat(rec.id, campaign_id, "space", "active", state.round as i32, "maneuver", &val)
                        .await?;
                    Ok(ToolExecutionResult {
                        output_json: json!({"message": msg, "range_band": state.range_band}).to_string(),
                        state_changed: true,
                    })
                }
                "fire" => {
                    let rec = db.get_active_combat(campaign_id, "space").await?.context("No active space combat")?;
                    let mut state: SpaceCombatState = serde_json::from_value(rec.state)?;
                    let atk_id = Uuid::parse_str(input.get("ship_id").and_then(|v| v.as_str()).context("ship_id required")?)?;
                    let tgt_id = Uuid::parse_str(input.get("target_ship_id").and_then(|v| v.as_str()).context("target_ship_id required")?)?;
                    let res = {
                        let mut r = rng();
                        resolve_space_attack(
                            &mut r,
                            &mut state,
                            &SpaceWeaponAttackInput {
                                attacker_ship_id: atk_id,
                                target_ship_id: tgt_id,
                                weapon_name: input.get("weapon_name").and_then(|v| v.as_str()).unwrap_or("Pulse Laser").into(),
                                linked_count: input.get("linked_count").and_then(|v| v.as_u64()).unwrap_or(1) as u8,
                                damage_dice: input.get("damage_dice").and_then(|v| v.as_u64()).unwrap_or(2) as u8,
                                is_laser: input.get("is_laser").and_then(|v| v.as_bool()).unwrap_or(true),
                                gunner_skill: input.get("gunner_skill").and_then(|v| v.as_i64()).unwrap_or(1) as i32,
                                gunner_dex_dm: input.get("gunner_dex_dm").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                                fire_control_dm: input.get("fire_control_dm").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                                sandcaster_count_defending: input.get("sandcaster_count").and_then(|v| v.as_u64()).unwrap_or(0) as u8,
                                defender_gunner_skill: 1,
                                called_shot: None,
                            },
                        )?
                    };
                    // Sync hull & crits back to ships table if target is a player ship
                    let mut owned = db.list_ships(campaign_id).await?;
                    if let Some(ship) = owned.iter_mut().find(|s| s.id == tgt_id) {
                        ship.hull_current = res.target_hull_remaining;
                        for cr in &res.crits_inflicted {
                            ship.crits.push(serde_json::to_value(cr)?);
                        }
                        db.save_ship(ship).await?;
                    }
                    let val = serde_json::to_value(&state)?;
                    db.upsert_combat(rec.id, campaign_id, "space", "active", state.round as i32, "attack", &val)
                        .await?;
                    Ok(ToolExecutionResult {
                        output_json: serde_json::to_string(&res)?,
                        state_changed: true,
                    })
                }
                "launch_missiles" => {
                    let rec = db.get_active_combat(campaign_id, "space").await?.context("No active space combat")?;
                    let mut state: SpaceCombatState = serde_json::from_value(rec.state)?;
                    let atk_id = Uuid::parse_str(input.get("ship_id").and_then(|v| v.as_str()).context("ship_id required")?)?;
                    let tgt_id = Uuid::parse_str(input.get("target_ship_id").and_then(|v| v.as_str()).context("target_ship_id required")?)?;
                    let salvo = launch_missile_salvo(
                        &mut state,
                        atk_id,
                        tgt_id,
                        input.get("missile_count").and_then(|v| v.as_u64()).unwrap_or(3) as u32,
                        input.get("smart_tl").and_then(|v| v.as_u64()).unwrap_or(12) as u8,
                    )?;
                    let val = serde_json::to_value(&state)?;
                    db.upsert_combat(rec.id, campaign_id, "space", "active", state.round as i32, "attack", &val)
                        .await?;
                    Ok(ToolExecutionResult {
                        output_json: serde_json::to_string(&salvo)?,
                        state_changed: true,
                    })
                }
                "ew_jam" => {
                    let rec = db.get_active_combat(campaign_id, "space").await?.context("No active space combat")?;
                    let mut state: SpaceCombatState = serde_json::from_value(rec.state)?;
                    let salvo_id = Uuid::parse_str(input.get("salvo_id").and_then(|v| v.as_str()).context("salvo_id required")?)?;
                    let (roll, destroyed) = {
                        let mut r = rng();
                        ew_jam_missiles(
                            &mut r,
                            &mut state,
                            salvo_id,
                            input.get("sensors_skill").and_then(|v| v.as_i64()).unwrap_or(1) as i32,
                            input.get("edu_dm").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                        )?
                    };
                    let val = serde_json::to_value(&state)?;
                    db.upsert_combat(rec.id, campaign_id, "space", "active", state.round as i32, "actions", &val)
                        .await?;
                    Ok(ToolExecutionResult {
                        output_json: json!({"roll": roll, "missiles_destroyed": destroyed}).to_string(),
                        state_changed: true,
                    })
                }
                "sensor_lock" => {
                    let rec = db.get_active_combat(campaign_id, "space").await?.context("No active space combat")?;
                    let mut state: SpaceCombatState = serde_json::from_value(rec.state)?;
                    let ship_id = Uuid::parse_str(input.get("ship_id").and_then(|v| v.as_str()).context("ship_id required")?)?;
                    let tgt_id = Uuid::parse_str(input.get("target_ship_id").and_then(|v| v.as_str()).context("target_ship_id required")?)?;
                    let roll = {
                        let mut r = rng();
                        acquire_sensor_lock(
                            &mut r,
                            &mut state,
                            ship_id,
                            tgt_id,
                            input.get("sensors_skill").and_then(|v| v.as_i64()).unwrap_or(1) as i32,
                            input.get("edu_dm").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                        )?
                    };
                    let val = serde_json::to_value(&state)?;
                    db.upsert_combat(rec.id, campaign_id, "space", "active", state.round as i32, "actions", &val)
                        .await?;
                    Ok(ToolExecutionResult {
                        output_json: json!({"roll": roll, "lock_acquired": roll.success}).to_string(),
                        state_changed: true,
                    })
                }
                "ai_turn" => {
                    let rec = db.get_active_combat(campaign_id, "space").await?.context("No active space combat")?;
                    let mut state: SpaceCombatState = serde_json::from_value(rec.state)?;
                    let sand = input.get("sandcaster_count").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
                    let gunner = input.get("gunner_skill").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
                    let report = {
                        let mut r = rng();
                        execute_space_ai_turn(&mut r, &mut state, sand, gunner)
                    };
                    // Sync party ship hull back to PostgreSQL ships table
                    let mut owned = db.list_ships(campaign_id).await?;
                    for cs in &state.ships {
                        if let Some(ship) = owned.iter_mut().find(|s| s.id == cs.ship_id) {
                            ship.hull_current = cs.hull_current;
                            db.save_ship(ship).await?;
                        }
                    }
                    let val = serde_json::to_value(&state)?;
                    db.upsert_combat(rec.id, campaign_id, "space", "active", state.round as i32, "actions", &val)
                        .await?;
                    Ok(ToolExecutionResult {
                        output_json: serde_json::to_string(&report)?,
                        state_changed: true,
                    })
                }
                "next_round" => {
                    let rec = db.get_active_combat(campaign_id, "space").await?.context("No active space combat")?;
                    let mut state: SpaceCombatState = serde_json::from_value(rec.state)?;
                    let auto_ai = input.get("auto_ai_turn").and_then(|v| v.as_bool()).unwrap_or(false);
                    let ai_report = if auto_ai {
                        let sand = input.get("sandcaster_count").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
                        let gunner = input.get("gunner_skill").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
                        Some({
                            let mut r = rng();
                            execute_space_ai_turn(&mut r, &mut state, sand, gunner)
                        })
                    } else {
                        None
                    };
                    let events = {
                        let mut r = rng();
                        advance_space_round(&mut r, &mut state)
                    };
                    let mut owned = db.list_ships(campaign_id).await?;
                    for cs in &state.ships {
                        if let Some(ship) = owned.iter_mut().find(|s| s.id == cs.ship_id) {
                            ship.hull_current = cs.hull_current;
                            db.save_ship(ship).await?;
                        }
                    }
                    let val = serde_json::to_value(&state)?;
                    db.upsert_combat(rec.id, campaign_id, "space", "active", state.round as i32, "maneuver", &val)
                        .await?;
                    Ok(ToolExecutionResult {
                        output_json: json!({"round": state.round, "ai_report": ai_report, "events": events, "state": state}).to_string(),
                        state_changed: true,
                    })
                }
                "end" => {
                    if let Some(rec) = db.get_active_combat(campaign_id, "space").await? {
                        db.upsert_combat(rec.id, campaign_id, "space", "ended", rec.round, "ended", &rec.state)
                            .await?;
                    }
                    Ok(ToolExecutionResult {
                        output_json: json!({"status": "ended"}).to_string(),
                        state_changed: true,
                    })
                }
                other => anyhow::bail!("Unknown space_combat action '{other}'"),
            }
        }

        "schedule_event" => {
            let camp = db
                .get_campaign(campaign_id)
                .await?
                .context("Campaign not found")?;
            let title = input
                .get("title")
                .and_then(|v| v.as_str())
                .context("title required")?;
            let kind = input.get("kind").and_then(|v| v.as_str()).unwrap_or("story");
            let delta = input.get("hours_from_now").and_then(|v| v.as_i64()).unwrap_or(0) * 60
                + input.get("days_from_now").and_then(|v| v.as_i64()).unwrap_or(0) * 1440;
            let due = camp.date.advance_minutes(delta.max(1));
            let ev = db
                .create_event(
                    campaign_id,
                    due.to_total_minutes(),
                    &due.format(),
                    kind,
                    title,
                    json!({}),
                )
                .await?;
            Ok(ToolExecutionResult {
                output_json: serde_json::to_string(&ev)?,
                state_changed: true,
            })
        }

        "advance_time" => {
            let camp = db
                .get_campaign(campaign_id)
                .await?
                .context("Campaign not found")?;
            let execute_jump = input
                .get("execute_jump")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let pay = input
                .get("pay_maintenance_periods")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let mut total_minutes = input.get("minutes").and_then(|v| v.as_i64()).unwrap_or(0)
                + input.get("hours").and_then(|v| v.as_i64()).unwrap_or(0) * 60
                + input.get("days").and_then(|v| v.as_i64()).unwrap_or(0) * 1440;

            let mut jump_hours = None;
            if execute_jump {
                let jh = {
                    let mut r = rng();
                    roll_jump_duration_hours(&mut r)
                };
                jump_hours = Some(jh);
                total_minutes += jh * 60;
            }
            if total_minutes <= 0 {
                anyhow::bail!("Time advance must be positive (minutes/hours/days or execute_jump=true)");
            }

            let adv = compute_advance(camp.date, total_minutes);
            db.update_campaign_date(campaign_id, adv.to).await?;
            let due_events = db
                .complete_due_events(campaign_id, adv.to.to_total_minutes())
                .await?;

            let mut billing_events = Vec::new();
            let accounts = db.list_accounts(campaign_id).await?;
            let company_acc = accounts
                .iter()
                .find(|a| a.kind == AccountKind::Company)
                .or_else(|| accounts.first());

            // 1. Extended Starport Berthing Fees: charge daily rate for any docked ship staying past its initial 6 prepaid days
            let mut ships = db.list_ships(campaign_id).await?;
            if let Ok(world) = parse_uwp(&camp.current_system_uwp) {
                let now_mins = adv.to.to_total_minutes();
                for ship in &mut ships {
                    if ship.is_docked
                        && let Some(docked_at) = ship.docked_at_total_minutes
                    {
                        let accrued = calculate_accrued_berthing_fee(
                            world.starport,
                            docked_at,
                            now_mins,
                            ship.berthing_billed_days.max(0) as u32,
                        );
                        if accrued.newly_billable_days > 0 {
                            ship.berthing_billed_days =
                                ship.berthing_billed_days.max(6) + accrued.newly_billable_days as i32;
                            db.save_ship(ship).await?;
                            if accrued.fee_cr > 0
                                && let Some(acc) = company_acc
                            {
                                let tx = db
                                    .post_transaction(
                                        campaign_id,
                                        acc.id,
                                        &adv.to.format_date_only(),
                                        -accrued.fee_cr,
                                        "berthing_fee",
                                        &format!(
                                            "Extended Starport {} berthing for {} ({} extra day(s) @ Cr{}/day; total stay {}d)",
                                            world.starport,
                                            ship.name,
                                            accrued.newly_billable_days,
                                            accrued.daily_rate_cr,
                                            accrued.total_days_docked
                                        ),
                                        &format!("{} Port Authority", camp.current_system_name),
                                    )
                                    .await?;
                                billing_events.push(json!({
                                    "kind": "extended_berthing",
                                    "ship": ship.name,
                                    "starport": world.starport.to_string(),
                                    "newly_billable_days": accrued.newly_billable_days,
                                    "total_days_docked": accrued.total_days_docked,
                                    "daily_rate_cr": accrued.daily_rate_cr,
                                    "total_cr": accrued.fee_cr,
                                    "already_debited_from_company_account": true,
                                    "balance_after_cr": tx.balance_after_cr
                                }));
                            }
                        }
                    }
                }
            }

            if adv.maintenance_periods_crossed > 0 {
                let chars = db.list_characters(campaign_id).await?;

                let crew_salaries: i64 = chars
                    .iter()
                    .filter(|r| r.character.kind == CharacterKind::Crew)
                    .map(|r| r.character.salary_cr)
                    .sum();
                let occupants = chars
                    .iter()
                    .filter(|r| {
                        matches!(r.character.kind, CharacterKind::Player | CharacterKind::Crew)
                    })
                    .count() as u32;

                for _ in 0..adv.maintenance_periods_crossed {
                    for ship in &mut ships {
                        let costs = compute_running_costs(
                            &ship.spec,
                            ship.mortgage.as_ref(),
                            occupants,
                            0,
                            crew_salaries,
                        );
                        if pay {
                            if let Some(acc) = company_acc {
                                let tx = db
                                    .post_transaction(
                                        campaign_id,
                                        acc.id,
                                        &adv.to.format_date_only(),
                                        -costs.total_cr,
                                        "maintenance_period",
                                        &format!(
                                            "{} 4-week costs (Mortgage Cr{}, Maint Cr{}, LifeSupp Cr{}, Salaries Cr{})",
                                            ship.name,
                                            costs.mortgage_cr,
                                            costs.maintenance_cr,
                                            costs.life_support_cr,
                                            costs.crew_salaries_cr
                                        ),
                                        "Starport & Bank",
                                    )
                                    .await?;
                                if let Some(m) = &mut ship.mortgage
                                    && m.active
                                    && m.periods_paid < m.periods_total
                                {
                                    m.periods_paid += 1;
                                }
                                ship.skipped_maintenance_periods = 0;
                                db.save_ship(ship).await?;
                                billing_events.push(json!({
                                    "ship": ship.name,
                                    "paid": true,
                                    "already_debited_from_company_account": true,
                                    "note": "Already posted to ledger and deducted from Company account. Do NOT call record_transaction again.",
                                    "total_cr": costs.total_cr,
                                    "balance_after_cr": tx.balance_after_cr
                                }));
                            }
                        } else {
                            ship.skipped_maintenance_periods += 1;
                            if let Some(m) = &mut ship.mortgage
                                && m.active
                            {
                                m.periods_overdue += 1;
                                m.arrears_cr += m.monthly_payment_cr;
                            }
                            let poor = {
                                let mut r = rng();
                                roll_poor_maintenance(
                                    &mut r,
                                    ship.skipped_maintenance_periods as u32,
                                )
                            };
                            if poor.suffered_crit {
                                ship.crits.push(json!({
                                    "source": "poor_maintenance",
                                    "date": adv.to.format_date_only(),
                                    "name": poor.crit_name,
                                    "effect": poor.effect
                                }));
                            }
                            db.save_ship(ship).await?;
                            billing_events.push(json!({
                                "ship": ship.name,
                                "paid": false,
                                "skipped_periods": ship.skipped_maintenance_periods,
                                "poor_maintenance_check": poor
                            }));
                        }
                    }
                }
            }

            Ok(ToolExecutionResult {
                output_json: serde_json::to_string(&json!({
                    "from": adv.from.format(),
                    "to": adv.to.format(),
                    "jump_hours": jump_hours,
                    "maintenance_periods_crossed": adv.maintenance_periods_crossed,
                    "annual_boundaries_crossed": adv.annual_boundaries_crossed,
                    "completed_events": due_events,
                    "billing_events": billing_events
                }))?,
                state_changed: true,
            })
        }

        "record_transaction" => {
            let camp = db
                .get_campaign(campaign_id)
                .await?
                .context("Campaign not found")?;
            let account_id = Uuid::parse_str(
                input
                    .get("account_id")
                    .and_then(|v| v.as_str())
                    .context("account_id required")?,
            )?;
            let amount_cr = input
                .get("amount_cr")
                .and_then(|v| v.as_i64())
                .context("amount_cr required")?;
            let category = input
                .get("category")
                .and_then(|v| v.as_str())
                .unwrap_or("general");
            let description = input
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let counterparty = input
                .get("counterparty")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let tx = db
                .post_transaction(
                    campaign_id,
                    account_id,
                    &camp.date.format_date_only(),
                    amount_cr,
                    category,
                    description,
                    counterparty,
                )
                .await?;
            Ok(ToolExecutionResult {
                output_json: serde_json::to_string(&tx)?,
                state_changed: true,
            })
        }

        "award_xp" => {
            let char_id = Uuid::parse_str(
                input
                    .get("character_id")
                    .and_then(|v| v.as_str())
                    .context("character_id required")?,
            )?;
            let amount = input.get("amount").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
            let skill = input.get("skill").and_then(|v| v.as_str());
            let mut row = db
                .get_character(campaign_id, char_id)
                .await?
                .context("Character not found")?;
            if let Some(sk) = skill {
                row.character.award_skill_xp(sk, amount);
            } else {
                row.character.general_xp += amount;
            }
            db.save_character(&row.character, &row.equipment).await?;
            Ok(ToolExecutionResult {
                output_json: serde_json::to_string(&json!({
                    "character": row.character.name,
                    "skill": skill,
                    "awarded": amount,
                    "general_xp": row.character.general_xp,
                    "skill_xp": row.character.skill_xp
                }))?,
                state_changed: true,
            })
        }

        "validate_ship" => {
            let ship_id = Uuid::parse_str(
                input
                    .get("ship_id")
                    .and_then(|v| v.as_str())
                    .context("ship_id required")?,
            )?;
            let ships = db.list_ships(campaign_id).await?;
            let ship = ships
                .into_iter()
                .find(|s| s.id == ship_id)
                .context("Ship not found")?;
            let report = validate_ship(&ship.spec);
            Ok(ToolExecutionResult {
                output_json: serde_json::to_string(&report)?,
                state_changed: false,
            })
        }

        "update_ledger" => {
            let mut camp = db
                .get_campaign(campaign_id)
                .await?
                .context("Campaign not found")?;
            let section = input
                .get("section")
                .and_then(|v| v.as_str())
                .context("section required")?;
            let text = input
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            if text.len() > 1800 {
                anyhow::bail!("Ledger section exceeds 1800 characters; condense and retry.");
            }
            match section {
                "who_knows_what" => camp.story_ledger.who_knows_what = text.to_string(),
                "settled" => camp.story_ledger.settled = text.to_string(),
                "people" => camp.story_ledger.people = text.to_string(),
                "threads" => camp.story_ledger.threads = text.to_string(),
                _ => anyhow::bail!("Unknown ledger section '{section}'"),
            }
            db.update_story_ledger(campaign_id, &camp.story_ledger).await?;
            Ok(ToolExecutionResult {
                output_json: format!("Ledger section '{section}' updated.\n{}", camp.story_ledger.render_xml()),
                state_changed: true,
            })
        }

        "supersede_last_reply" => {
            let mut camp = db
                .get_campaign(campaign_id)
                .await?
                .context("Campaign not found")?;
            let correction = input
                .get("correction")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            if correction.is_empty() {
                anyhow::bail!("'correction' is required");
            }
            let Some(prev) = db.supersede_last_assistant_message(campaign_id).await? else {
                anyhow::bail!("No earlier Referee reply to withdraw.");
            };
            let deleted = hindsight
                .delete_message(&camp.hindsight_bank_id, &prev.content_hash)
                .await
                .unwrap_or(false);
            let line = format!("- {correction}");
            if !camp.story_ledger.settled.contains(&line) {
                if !camp.story_ledger.settled.is_empty() {
                    camp.story_ledger.settled.push('\n');
                }
                camp.story_ledger.settled.push_str(&line);
                db.update_story_ledger(campaign_id, &camp.story_ledger).await?;
            }
            Ok(ToolExecutionResult {
                output_json: format!(
                    "Previous reply withdrawn (hindsight_deleted={deleted}); correction locked in settled facts. Write the corrected narrative now."
                ),
                state_changed: true,
            })
        }

        "system_traffic" => {
            let camp = db
                .get_campaign(campaign_id)
                .await?
                .context("Campaign not found")?;
            let action = input
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("list");

            match action {
                "list" => {
                    let mut ships = db
                        .list_system_ships(campaign_id, &camp.current_system_name)
                        .await?;
                    if ships.is_empty() {
                        let sys_rec = db.get_system_by_name(&camp.current_system_name).await?;
                        let uwp = parse_uwp(&camp.current_system_uwp)
                            .unwrap_or_else(|_| parse_uwp("C555555-9").expect("valid uwp"));
                        let (bases, zone) = sys_rec
                            .map(|s| (s.bases, s.travel_zone))
                            .unwrap_or_else(|| (vec![], "Green".to_string()));
                        let report = {
                            let mut r = rng();
                            generate_system_traffic(
                                &mut r,
                                &camp.current_system_name,
                                &uwp,
                                &bases,
                                &zone,
                            )
                        };
                        ships = db
                            .replace_system_ships(
                                campaign_id,
                                &camp.current_system_name,
                                &report.vessels,
                            )
                            .await?;
                    }
                    Ok(ToolExecutionResult {
                        output_json: serde_json::to_string(&json!({
                            "system": camp.current_system_name,
                            "in_system_ship_count": ships.len(),
                            "ships": ships
                        }))?,
                        state_changed: false,
                    })
                }
                "scan_system" => {
                    let sys_rec = db.get_system_by_name(&camp.current_system_name).await?;
                    let uwp = parse_uwp(&camp.current_system_uwp)
                        .unwrap_or_else(|_| parse_uwp("C555555-9").expect("valid uwp"));
                    let (bases, zone) = sys_rec
                        .map(|s| (s.bases, s.travel_zone))
                        .unwrap_or_else(|| (vec![], "Green".to_string()));
                    let report = {
                        let mut r = rng();
                        generate_system_traffic(
                            &mut r,
                            &camp.current_system_name,
                            &uwp,
                            &bases,
                            &zone,
                        )
                    };
                    let ships = db
                        .replace_system_ships(
                            campaign_id,
                            &camp.current_system_name,
                            &report.vessels,
                        )
                        .await?;
                    Ok(ToolExecutionResult {
                        output_json: serde_json::to_string(&json!({
                            "system": camp.current_system_name,
                            "in_system_ship_count": ships.len(),
                            "summary": report.summary,
                            "ships": ships
                        }))?,
                        state_changed: true,
                    })
                }
                "hail" => {
                    let vessel_id = Uuid::parse_str(
                        input
                            .get("vessel_id")
                            .and_then(|v| v.as_str())
                            .context("vessel_id required")?,
                    )?;
                    let msg = input
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Opening hailing frequencies.");
                    let disp = input.get("disposition").and_then(|v| v.as_str());
                    let entry = json!({
                        "date": camp.date.format(),
                        "from": "player",
                        "message": msg
                    });
                    let vessel = db
                        .update_system_ship(campaign_id, vessel_id, disp, None, Some(entry))
                        .await?
                        .context("In-system vessel not found")?;
                    Ok(ToolExecutionResult {
                        output_json: serde_json::to_string(&json!({
                            "hailed": vessel.name,
                            "captain": vessel.captain_name,
                            "faction": vessel.faction,
                            "disposition": vessel.disposition,
                            "orbit": vessel.orbit_location,
                            "transponder_active": vessel.transponder_active,
                            "cargo_hint": vessel.cargo_hint,
                            "vessel": vessel
                        }))?,
                        state_changed: true,
                    })
                }
                "sensor_scan" => {
                    let vessel_id = Uuid::parse_str(
                        input
                            .get("vessel_id")
                            .and_then(|v| v.as_str())
                            .context("vessel_id required")?,
                    )?;
                    let vessel = db
                        .get_system_ship(campaign_id, vessel_id)
                        .await?
                        .context("In-system vessel not found")?;
                    let player_ships = db.list_ships(campaign_id).await?;
                    let (obs_grade, obs_tl) = player_ships
                        .first()
                        .map(|s| (s.spec.sensor_grade, s.spec.tl))
                        .unwrap_or((SensorGrade::Civilian, 12));
                    let det = {
                        let mut r = rng();
                        resolve_sensor_detection(
                            &mut r,
                            &SensorDetectionInput {
                                context: SensorContext::SpaceInitialDetection,
                                operator_edu_dm: 1,
                                operator_sensors_skill: 1,
                                augment_or_software_dm: 0,
                                observer_sensor_grade: obs_grade,
                                observer_tl: obs_tl,
                                observer_extended_array: false,
                                observer_active_sensors: true,
                                target_tl: vessel.tl.max(0) as u8,
                                target_active_sensors: vessel.transponder_active,
                                target_thrust_applied: if vessel.transponder_active { 1 } else { 0 },
                                target_power_plant_active: true,
                                target_transponder_or_comms: vessel.transponder_active,
                                target_stealth_armor_dm: i32::from(vessel.stealth_dm),
                                target_fired_weapons: false,
                                target_crit_damage_severity: if vessel.hull_current < vessel.hull_max { 1 } else { 0 },
                                situational_dm: 0,
                                roll_mode: RollMode::Normal,
                            },
                        )
                    };
                    Ok(ToolExecutionResult {
                        output_json: serde_json::to_string(&json!({
                            "target": vessel.name,
                            "ship_class": vessel.ship_class,
                            "sensor_detection": det,
                            "revealed": if det.roll.success {
                                json!({
                                    "tonnage": vessel.tonnage,
                                    "hull": format!("{}/{}", vessel.hull_current, vessel.hull_max),
                                    "armor": vessel.armor,
                                    "thrust": vessel.thrust,
                                    "stealth_dm": vessel.stealth_dm,
                                    "cargo_hint": vessel.cargo_hint
                                })
                            } else {
                                json!({"note": "Target signature too faint / stealthed for detailed scan."})
                            }
                        }))?,
                        state_changed: false,
                    })
                }
                "engage" => {
                    let vessel_id = Uuid::parse_str(
                        input
                            .get("vessel_id")
                            .and_then(|v| v.as_str())
                            .context("vessel_id required")?,
                    )?;
                    let vessel = db
                        .update_system_ship(campaign_id, vessel_id, Some("hostile"), None, None)
                        .await?
                        .context("In-system vessel not found")?;
                    let mut player_ships = db.list_ships(campaign_id).await?;
                    for ps_mut in &mut player_ships {
                        if ps_mut.is_docked {
                            ps_mut.is_docked = false;
                            ps_mut.docked_at_total_minutes = None;
                            ps_mut.berthing_billed_days = 0;
                            db.save_ship(ps_mut).await?;
                        }
                    }
                    let ps = player_ships.first();
                    let range = input
                        .get("range_band")
                        .and_then(|v| v.as_str())
                        .unwrap_or("long");
                    let start_body = json!({
                        "action": "start",
                        "range_band": range,
                        "ships": [
                            {
                                "ship_id": ps.map(|s| s.id.to_string()),
                                "name": ps.map(|s| s.name.as_str()).unwrap_or("Player Ship"),
                                "side": "party",
                                "tonnage": ps.map(|s| s.spec.tonnage).unwrap_or(200),
                                "tl": ps.map(|s| s.spec.tl).unwrap_or(12),
                                "hull_max": ps.map(|s| s.spec.hull_points_max).unwrap_or(80),
                                "hull_current": ps.map(|s| s.hull_current).unwrap_or(80),
                                "armor": ps.map(|s| s.spec.armor_rating).unwrap_or(4),
                                "thrust_max": ps.map(|s| s.spec.thrust).unwrap_or(2),
                                "max_weapons_per_round": (ps.map(|s| s.spec.tonnage).unwrap_or(200) / 200).max(1)
                            },
                            {
                                "ship_id": vessel.id.to_string(),
                                "name": vessel.name,
                                "side": "hostile",
                                "tonnage": vessel.tonnage,
                                "tl": vessel.tl,
                                "hull_max": vessel.hull_max,
                                "hull_current": vessel.hull_current,
                                "armor": vessel.armor,
                                "thrust_max": vessel.thrust,
                                "max_weapons_per_round": (vessel.tonnage / 200).max(1)
                            }
                        ]
                    });
                    Box::pin(execute_tool(db, hindsight, campaign_id, "space_combat", &start_body))
                        .await
                }
                "update_status" => {
                    let vessel_id = Uuid::parse_str(
                        input
                            .get("vessel_id")
                            .and_then(|v| v.as_str())
                            .context("vessel_id required")?,
                    )?;
                    let vessel = db
                        .update_system_ship(
                            campaign_id,
                            vessel_id,
                            input.get("disposition").and_then(|v| v.as_str()),
                            input.get("status").and_then(|v| v.as_str()),
                            None,
                        )
                        .await?
                        .context("In-system vessel not found")?;
                    Ok(ToolExecutionResult {
                        output_json: serde_json::to_string(&json!({ "vessel": vessel }))?,
                        state_changed: true,
                    })
                }
                other => anyhow::bail!("Unknown system_traffic action '{other}'"),
            }
        }

        "port_ops" => {
            let camp = db
                .get_campaign(campaign_id)
                .await?
                .context("Campaign not found")?;
            let action = input.get("action").and_then(|v| v.as_str()).unwrap_or("refuel");
            let mut ships = db.list_ships(campaign_id).await?;
            let ship_id_opt = input
                .get("ship_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok());
            let ship = if let Some(sid) = ship_id_opt {
                ships
                    .iter_mut()
                    .find(|s| s.id == sid)
                    .context("Ship not found")?
            } else {
                ships.first_mut().context("No ship owned")?
            };
            let world = parse_uwp(&camp.current_system_uwp)?;
            let sys_rec = db.get_system_by_name(&camp.current_system_name).await.ok().flatten();
            let gas_giants = sys_rec.map(|s| s.gas_giants).unwrap_or(1);

            match action {
                "refuel" => {
                    let mode = match input.get("mode").and_then(|v| v.as_str()).unwrap_or("refined") {
                        "unrefined" => RefuelMode::Unrefined,
                        "gas_giant_skim" => RefuelMode::GasGiantSkim,
                        "process_onboard" => RefuelMode::ProcessOnboard,
                        _ => RefuelMode::Refined,
                    };
                    let free_tank = (ship.spec.fuel_capacity_tons - (ship.fuel_refined + ship.fuel_unrefined)).max(0.0);
                    let requested = input
                        .get("tons")
                        .and_then(|v| v.as_f64())
                        .unwrap_or_else(|| {
                            if mode == RefuelMode::ProcessOnboard {
                                ship.fuel_unrefined
                            } else {
                                free_tank
                            }
                        });
                    let actual_tons = if mode == RefuelMode::ProcessOnboard {
                        requested.min(ship.fuel_unrefined).max(0.0)
                    } else {
                        requested.min(free_tank).max(0.0)
                    };
                    if actual_tons <= 0.001 {
                        anyhow::bail!(
                            "{} has no fuel space or unrefined fuel to process (Refined {:.1}t / Unrefined {:.1}t / Cap {:.1}t).",
                            ship.name,
                            ship.fuel_refined,
                            ship.fuel_unrefined,
                            ship.spec.fuel_capacity_tons
                        );
                    }
                    if matches!(mode, RefuelMode::Refined | RefuelMode::Unrefined) && !ship.is_docked {
                        anyhow::bail!(
                            "{} must dock at {} Starport before purchasing commercial starport fuel.",
                            ship.name,
                            camp.current_system_name
                        );
                    }
                    if matches!(mode, RefuelMode::GasGiantSkim) && ship.is_docked {
                        anyhow::bail!(
                            "{} must undock from {} Starport before skimming a gas giant.",
                            ship.name,
                            camp.current_system_name
                        );
                    }
                    let quote = calculate_refuel_quote(world.starport, gas_giants, &ship.spec, actual_tons, mode);
                    if !quote.allowed {
                        anyhow::bail!("{}", quote.note);
                    }
                    if quote.total_cost_cr > 0 {
                        let accounts = db.list_accounts(campaign_id).await?;
                        let company = accounts
                            .iter()
                            .find(|a| a.kind == AccountKind::Company)
                            .or_else(|| accounts.first())
                            .context("No account found")?;
                        db.post_transaction(
                            campaign_id,
                            company.id,
                            &camp.date.format_date_only(),
                            -quote.total_cost_cr,
                            "fuel",
                            &format!("{} — {}", ship.name, quote.note),
                            &format!("{} Starport", camp.current_system_name),
                        )
                        .await?;
                    }
                    match mode {
                        RefuelMode::Refined => ship.fuel_refined += actual_tons,
                        RefuelMode::Unrefined | RefuelMode::GasGiantSkim => ship.fuel_unrefined += actual_tons,
                        RefuelMode::ProcessOnboard => {
                            ship.fuel_unrefined = (ship.fuel_unrefined - actual_tons).max(0.0);
                            ship.fuel_refined += actual_tons;
                        }
                    }
                    db.save_ship(ship).await?;
                    if quote.hours_required > 0 {
                        let _ = Box::pin(execute_tool(
                            db,
                            hindsight,
                            campaign_id,
                            "advance_time",
                            &json!({"hours": quote.hours_required, "pay_maintenance_periods": true}),
                        ))
                        .await;
                    }
                    Ok(ToolExecutionResult {
                        output_json: json!({
                            "ship": ship.name,
                            "quote": quote,
                            "fuel_refined": ship.fuel_refined,
                            "fuel_unrefined": ship.fuel_unrefined
                        })
                        .to_string(),
                        state_changed: true,
                    })
                }
                "dock" => {
                    let sp = world.starport.to_ascii_uppercase();
                    if sp == 'X' {
                        anyhow::bail!(
                            "Starport Class X at {} has no starport facilities to dock at.",
                            camp.current_system_name
                        );
                    }
                    if ship.is_docked
                        && ship
                            .docked_system_name
                            .eq_ignore_ascii_case(&camp.current_system_name)
                    {
                        anyhow::bail!(
                            "{} is already docked at {} Starport. Extended stay fees accrue automatically as days pass; undock first if you wish to depart.",
                            ship.name,
                            camp.current_system_name
                        );
                    }
                    let fee = {
                        let mut r = rng();
                        roll_berthing_fee(&mut r, sp, 0)
                    };
                    if fee.total_cr > 0 {
                        let accounts = db.list_accounts(campaign_id).await?;
                        let company = accounts
                            .iter()
                            .find(|a| a.kind == AccountKind::Company)
                            .or_else(|| accounts.first())
                            .context("No account found")?;
                        let available = company.balance_cr + company.credit_limit_cr;
                        if available < fee.total_cr {
                            anyhow::bail!(
                                "Insufficient funds in {} to pay Starport {} docking fee (need Cr{}, available Cr{}).",
                                company.name,
                                sp,
                                fee.total_cr,
                                available
                            );
                        }
                        db.post_transaction(
                            campaign_id,
                            company.id,
                            &camp.date.format_date_only(),
                            -fee.total_cr,
                            "berthing_fee",
                            &format!(
                                "Starport {} docking fee for {} (1D={} -> Cr{}, covers first 6 days)",
                                sp, ship.name, fee.roll_1d6, fee.base_berthing_cr
                            ),
                            &format!("{} Port Authority", camp.current_system_name),
                        )
                        .await?;
                    }
                    ship.is_docked = true;
                    ship.docked_system_name = camp.current_system_name.clone();
                    ship.docked_at_total_minutes = Some(camp.date.to_total_minutes());
                    ship.berthing_billed_days = 6;
                    db.save_ship(ship).await?;
                    let extra_day_rate_cr = berthing_extra_day_rate(sp);
                    Ok(ToolExecutionResult {
                        output_json: json!({
                            "ship": ship.name,
                            "system": camp.current_system_name,
                            "is_docked": true,
                            "berthing_fee": fee,
                            "extra_day_rate_cr": extra_day_rate_cr,
                            "note": format!(
                                "Docked {} at {} Starport {}. Initial fee Cr{} covers days 1–6; extra days accrue at Cr{}/day automatically.",
                                ship.name, camp.current_system_name, sp, fee.total_cr, extra_day_rate_cr
                            )
                        })
                        .to_string(),
                        state_changed: true,
                    })
                }
                "undock" => {
                    if !ship.is_docked {
                        anyhow::bail!("{} is already in orbit (not docked at a starport).", ship.name);
                    }
                    let days_stayed = ship
                        .docked_at_total_minutes
                        .map(|t0| (camp.date.to_total_minutes() - t0).max(0) / 1440)
                        .unwrap_or(0);
                    ship.is_docked = false;
                    ship.docked_at_total_minutes = None;
                    ship.berthing_billed_days = 0;
                    db.save_ship(ship).await?;
                    Ok(ToolExecutionResult {
                        output_json: json!({
                            "ship": ship.name,
                            "system": camp.current_system_name,
                            "is_docked": false,
                            "days_stayed": days_stayed,
                            "note": format!(
                                "{} undocked from {} Starport after {} day(s) and broke orbit.",
                                ship.name, camp.current_system_name, days_stayed
                            )
                        })
                        .to_string(),
                        state_changed: true,
                    })
                }
                other => anyhow::bail!("Unknown port_ops action '{other}'"),
            }
        }

        "recall_memory" => {
            let camp = db
                .get_campaign(campaign_id)
                .await?
                .context("Campaign not found")?;
            let query = input
                .get("query")
                .and_then(|v| v.as_str())
                .context("query required")?;
            let sys_rec = db.get_system_by_name(&camp.current_system_name).await.ok().flatten();
            let uwp = parse_uwp(&camp.current_system_uwp)
                .unwrap_or_else(|_| parse_uwp("A788899-C").expect("valid uwp"));
            let scope = build_spatial_memory_scope(
                &camp.current_system_name,
                sys_rec.as_ref().map(|s| s.sector_name.as_str()),
                &uwp,
                sys_rec.as_ref().map(|s| s.allegiance.as_str()).unwrap_or("Im"),
            );
            let recalled = hindsight
                .recall_scoped(
                    &camp.hindsight_bank_id,
                    query,
                    &scope.local_tags,
                    &scope.polity_tags,
                    2048,
                )
                .await?;
            Ok(ToolExecutionResult {
                output_json: if recalled.trim().is_empty() {
                    "No matching memories found in Hindsight.".to_string()
                } else {
                    recalled
                },
                state_changed: false,
            })
        }

        other => anyhow::bail!("Unknown tool '{other}'"),
    }
}
