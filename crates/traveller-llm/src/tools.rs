use anyhow::{Context, Result};
use rand::rng;
use serde_json::{json, Value};
use traveller_db::Db;
use traveller_rules::calendar::{compute_advance, roll_jump_duration_hours};
use traveller_rules::character::{Characteristic, CharacterKind};
use traveller_rules::dice::{resolve_task, roll_d66, roll_nd6, Difficulty, ModifierItem, RollMode};
use traveller_rules::sensors::{resolve_sensor_detection, SensorContext, SensorDetectionInput};
use traveller_rules::ship::{
    compute_running_costs, roll_poor_maintenance, validate_ship, SensorGrade,
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
                    "operator_character_id": {"type": "string", "description": "Optional character UUID to pull EDU DM, Electronics (sensors), and Neural Jack bonuses automatically."},
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
            "name": "advance_time",
            "description": "Advance the Imperial Calendar (DDD-YYYY HH:MM) by minutes/hours/days or execute a 1-week Jump (148 + 6D6 hours). When a 28-day Maintenance Period passes and pay_maintenance_periods=true, this tool AUTOMATICALLY debits ship mortgages, 0.1%/yr maintenance, life support, and crew salaries from the Company account — NEVER call record_transaction again for those bills.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "minutes": {"type": "integer", "default": 0},
                    "hours": {"type": "integer", "default": 0},
                    "days": {"type": "integer", "default": 0},
                    "execute_jump": {"type": "boolean", "default": false, "description": "Roll 148 + 6D6 hours jump transit."},
                    "pay_maintenance_periods": {"type": "boolean", "default": true, "description": "If false when a 28-day Maintenance Period passes, skips payment and triggers arrears / Poor Maintenance checks."}
                }
            }
        }),
        json!({
            "name": "record_transaction",
            "description": "Debit or credit a Personal or Company account and record a ledger transaction. Do NOT call this for 28-day Maintenance Periods (mortgage/maintenance/life support/salaries) that were already paid automatically by advance_time.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "account_id": {"type": "string", "description": "UUID of the target account."},
                    "amount_cr": {"type": "integer", "description": "Positive for income/deposit, negative for expense."},
                    "category": {"type": "string", "description": "e.g. trade, fuel, berthing, purchase, salary, mortgage, maintenance, transfer."},
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
                    "skill": {"type": "string", "description": "Skill name for per-skill Dedicated XP. Omit to award General XP."},
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
                    "text": {"type": "string", "description": "Full concise replacement text for this section (max 1800 chars)."}
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
                    "correction": {"type": "string", "description": "One-sentence corrected fact to lock into settled facts."}
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
                    "query": {"type": "string", "description": "What to recall from campaign history."}
                },
                "required": ["query"]
            }
        }));
    }

    // Place prompt-cache breakpoint on the final tool definition (anthropic-pipe parity)
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
            if award_xp
                && let Some(sk) = skill {
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
            let mut r = rng();
            let out = if kind == "d66" {
                let d66 = roll_d66(&mut r);
                json!({"kind": "d66", "result": d66, "reason": input.get("reason")})
            } else {
                let count = input.get("count").and_then(|v| v.as_u64()).unwrap_or(2).clamp(1, 20) as usize;
                let dice = roll_nd6(&mut r, count);
                let sum: i32 = dice.iter().map(|&d| i32::from(d)).sum();
                json!({"kind": format!("{count}D6"), "dice": dice, "dm": dm, "total": sum + dm, "reason": input.get("reason")})
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
                    && let Some(row) = db.get_character(campaign_id, cid).await? {
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

            let mut r = rng();
            let res = resolve_sensor_detection(&mut r, &req);
            Ok(ToolExecutionResult {
                output_json: serde_json::to_string(&res)?,
                state_changed: false,
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

            let mut billing_events = Vec::new();
            if adv.maintenance_periods_crossed > 0 {
                let chars = db.list_characters(campaign_id).await?;
                let mut ships = db.list_ships(campaign_id).await?;
                let accounts = db.list_accounts(campaign_id).await?;
                let company_acc = accounts
                    .iter()
                    .find(|a| a.kind == traveller_rules::finance::AccountKind::Company);

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
                                    && m.active && m.periods_paid < m.periods_total {
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
                                && m.active {
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

        "recall_memory" => {
            let camp = db
                .get_campaign(campaign_id)
                .await?
                .context("Campaign not found")?;
            let query = input
                .get("query")
                .and_then(|v| v.as_str())
                .context("query required")?;
            let recalled = hindsight.recall(&camp.hindsight_bank_id, query, 2048).await?;
            Ok(ToolExecutionResult {
                output_json: recalled,
                state_changed: false,
            })
        }

        other => anyhow::bail!("Unknown tool '{other}'"),
    }
}
