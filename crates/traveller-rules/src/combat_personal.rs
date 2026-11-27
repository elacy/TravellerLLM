use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::character::{characteristic_dm, Character, Characteristic};
use crate::dice::{resolve_task, roll_nd6, DiceRollResult, Difficulty, ModifierItem, RollMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonalRangeBand {
    Adjacent, // <1.5m (Melee)
    Close,    // 1.5–5m (+1 DM for pistols/shotguns)
    Short,    // 6–12m (+1 DM)
    Medium,   // 13–50m (0)
    Long,     // 51–250m (-2)
    VeryLong, // 251–500m (-4)
    Distant,  // 501m+ (-6)
}

impl PersonalRangeBand {
    pub fn attack_dm(self, weapon_range_m: u32) -> i32 {
        let band_m = match self {
            Self::Adjacent => 1,
            Self::Close => 3,
            Self::Short => 10,
            Self::Medium => 35,
            Self::Long => 150,
            Self::VeryLong => 400,
            Self::Distant => 800,
        };
        if band_m <= weapon_range_m / 4 {
            1 // Short range (+1)
        } else if band_m <= weapon_range_m {
            0 // Effective range
        } else if band_m <= weapon_range_m * 2 {
            -2 // Long range (-2)
        } else if band_m <= weapon_range_m * 4 {
            -4 // Extreme range (-4)
        } else {
            -6
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReactionKind {
    None,
    Dodge,        // -1 next round initiative, imposes -DEX/Athletics DM on incoming attack
    Parry,        // Melee only, -1 next round initiative, imposes -Melee DM
    DiveForCover, // -2 next round initiative, imposes -2 DM on incoming ranged attack
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalWeapon {
    pub name: String,
    pub skill: String, // e.g. "Gun Combat (Slug)", "Melee (Blade)"
    pub range_m: u32,
    pub damage_dice: u8,
    pub damage_bonus: i32,
    pub ap: i32,
    pub auto_rating: u8,
    pub stun: bool,
    pub magazine_current: u32,
    pub magazine_max: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalCombatant {
    pub character_id: Uuid,
    pub name: String,
    pub side: String, // "party" or "hostile"
    pub initiative: i32,
    pub next_round_init_penalty: i32,
    pub armor: i32,
    pub cover_dm: i32, // 0, -2 partial, -4 full
    pub aim_bonus: i32,
    pub range_band: PersonalRangeBand,
    pub weapon: PersonalWeapon,
    pub conscious: bool,
    pub alive: bool,
    pub str_cur: i32,
    pub dex_cur: i32,
    pub end_cur: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalCombatState {
    pub id: Uuid,
    pub round: u32,
    pub active_index: usize,
    pub combatants: Vec<PersonalCombatant>,
    pub log: Vec<String>,
}

/// Core 2022 p. 73: Start Personal Combat (6-second rounds), rolling 2D6 + DEX DM + Tactics bonus.
pub fn start_personal_combat<R: Rng + ?Sized>(
    rng: &mut R,
    mut combatants: Vec<PersonalCombatant>,
    tactics_bonus_party: i32,
    tactics_bonus_hostile: i32,
) -> PersonalCombatState {
    for c in &mut combatants {
        let d = roll_nd6(rng, 2);
        let t_dm = if c.side == "party" {
            tactics_bonus_party
        } else {
            tactics_bonus_hostile
        };
        c.initiative = i32::from(d[0]) + i32::from(d[1]) + characteristic_dm(c.dex_cur) + t_dm;
    }
    combatants.sort_by_key(|a| std::cmp::Reverse(a.initiative));

    PersonalCombatState {
        id: Uuid::new_v4(),
        round: 1,
        active_index: 0,
        combatants,
        log: vec!["Round 1 started (6-second personal combat).".into()],
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalAttackInput {
    pub attacker_id: Uuid,
    pub defender_id: Uuid,
    pub burst_mode: bool,
    pub reaction: ReactionKind,
    pub defender_reaction_dm: i32, // e.g. Athletics (Dex) or Melee skill level
    pub extra_dm: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalAttackResult {
    pub attack_roll: DiceRollResult,
    pub hit: bool,
    pub raw_damage: i32,
    pub armor_applied: i32,
    pub net_damage: i32,
    pub end_lost: i32,
    pub dex_lost: i32,
    pub str_lost: i32,
    pub defender_conscious: bool,
    pub defender_alive: bool,
    pub ammo_remaining: u32,
    pub summary: String,
}

/// Resolve a Core 2022 personal combat attack: ammo consumption, range/cover/aim/reaction DMs,
/// Weapon Dice + Effect - max(0, Armor - AP), damage applied END -> DEX/STR, and unconsciousness/death.
pub fn resolve_personal_attack<R: Rng + ?Sized>(
    rng: &mut R,
    state: &mut PersonalCombatState,
    attacker_char: Option<&Character>,
    input: &PersonalAttackInput,
) -> anyhow::Result<PersonalAttackResult> {
    let atk_idx = state
        .combatants
        .iter()
        .position(|c| c.character_id == input.attacker_id)
        .ok_or_else(|| anyhow::anyhow!("Attacker not in combat"))?;
    let def_idx = state
        .combatants
        .iter()
        .position(|c| c.character_id == input.defender_id)
        .ok_or_else(|| anyhow::anyhow!("Defender not in combat"))?;

    let weapon = state.combatants[atk_idx].weapon.clone();
    let rounds_fired = if input.burst_mode && weapon.auto_rating > 0 {
        u32::from(weapon.auto_rating)
    } else if weapon.magazine_max > 0 {
        1
    } else {
        0
    };
    if weapon.magazine_max > 0 && state.combatants[atk_idx].weapon.magazine_current < rounds_fired {
        anyhow::bail!("Out of ammunition in {} — reload required!", weapon.name);
    }
    state.combatants[atk_idx].weapon.magazine_current -= rounds_fired;
    let ammo_remaining = state.combatants[atk_idx].weapon.magazine_current;

    let mut mods = if let Some(ch) = attacker_char {
        ch.task_modifiers(Some(&weapon.skill), Characteristic::Dex, Difficulty::Average)
    } else {
        vec![ModifierItem {
            label: "DEX DM".into(),
            value: characteristic_dm(state.combatants[atk_idx].dex_cur),
        }]
    };

    let range_dm = state.combatants[def_idx].range_band.attack_dm(weapon.range_m);
    if range_dm != 0 {
        mods.push(ModifierItem { label: "Range band".into(), value: range_dm });
    }
    let cover_dm = state.combatants[def_idx].cover_dm;
    if cover_dm != 0 {
        mods.push(ModifierItem { label: "Target cover".into(), value: cover_dm });
    }
    let aim = state.combatants[atk_idx].aim_bonus;
    if aim > 0 {
        mods.push(ModifierItem { label: "Aim bonus".into(), value: aim });
        state.combatants[atk_idx].aim_bonus = 0;
    }

    match input.reaction {
        ReactionKind::Dodge => {
            let pen = -(input.defender_reaction_dm.max(1));
            mods.push(ModifierItem { label: "Defender Dodge".into(), value: pen });
            state.combatants[def_idx].next_round_init_penalty += 1;
        }
        ReactionKind::Parry => {
            let pen = -(input.defender_reaction_dm.max(1));
            mods.push(ModifierItem { label: "Defender Parry".into(), value: pen });
            state.combatants[def_idx].next_round_init_penalty += 1;
        }
        ReactionKind::DiveForCover => {
            mods.push(ModifierItem { label: "Dive for Cover".into(), value: -2 });
            state.combatants[def_idx].next_round_init_penalty += 2;
        }
        ReactionKind::None => {}
    }
    if input.extra_dm != 0 {
        mods.push(ModifierItem { label: "Situational DM".into(), value: input.extra_dm });
    }

    let attack_roll = resolve_task(rng, RollMode::Normal, Difficulty::Average, mods);
    let mut raw_damage = 0;
    let mut armor_applied = 0;
    let mut net_damage = 0;
    let mut end_lost = 0;
    let mut dex_lost = 0;
    let mut str_lost = 0;

    if attack_roll.success {
        let dice_sum: i32 = roll_nd6(rng, usize::from(weapon.damage_dice))
            .into_iter()
            .map(i32::from)
            .sum();
        let burst_bonus = if input.burst_mode { i32::from(weapon.auto_rating) } else { 0 };
        raw_damage = dice_sum + weapon.damage_bonus + burst_bonus + attack_roll.effect;
        armor_applied = (state.combatants[def_idx].armor - weapon.ap).max(0);
        net_damage = (raw_damage - armor_applied).max(0);

        if net_damage > 0 {
            let def = &mut state.combatants[def_idx];
            if weapon.stun {
                // Core p. 79 Stun: END check (8+) vs damage or unconscious
                let stun_check = roll_nd6(rng, 2);
                let stun_total =
                    i32::from(stun_check[0]) + i32::from(stun_check[1]) + characteristic_dm(def.end_cur) - net_damage;
                if stun_total < 8 {
                    def.conscious = false;
                }
            } else {
                // Core p. 76: Apply damage to END first, then overflow to DEX then STR
                let mut rem = net_damage;
                end_lost = rem.min(def.end_cur);
                def.end_cur -= end_lost;
                rem -= end_lost;

                if rem > 0 {
                    dex_lost = rem.min(def.dex_cur);
                    def.dex_cur -= dex_lost;
                    rem -= dex_lost;
                }
                if rem > 0 {
                    str_lost = rem.min(def.str_cur);
                    def.str_cur -= str_lost;
                }

                // Core p. 76: 2 physical stats at 0 -> unconscious; all 3 at 0 -> dead
                let zero_stats = [def.end_cur, def.dex_cur, def.str_cur]
                    .into_iter()
                    .filter(|&v| v <= 0)
                    .count();
                if zero_stats >= 3 {
                    def.conscious = false;
                    def.alive = false;
                } else if zero_stats >= 2 {
                    def.conscious = false;
                }
            }
        }
    }

    let def = &state.combatants[def_idx];
    let atk_name = state.combatants[atk_idx].name.clone();
    let def_name = def.name.clone();
    let conscious = def.conscious;
    let alive = def.alive;

    let summary = if attack_roll.success {
        format!(
            "{atk_name} hits {def_name} with {} (Effect +{}) for {net_damage} net dmg ({raw_damage} raw - {armor_applied} armor; END -{end_lost}, DEX -{dex_lost}, STR -{str_lost}).",
            weapon.name, attack_roll.effect
        )
    } else {
        format!(
            "{atk_name} misses {def_name} with {} (Effect {}).",
            weapon.name, attack_roll.effect
        )
    };
    state.log.push(summary.clone());

    let hit = net_damage > 0 || (attack_roll.success && weapon.stun);
    Ok(PersonalAttackResult {
        attack_roll,
        hit,
        raw_damage,
        armor_applied,
        net_damage,
        end_lost,
        dex_lost,
        str_lost,
        defender_conscious: conscious,
        defender_alive: alive,
        ammo_remaining,
        summary,
    })
}

/// Advance to the next 6-second personal combat round, applying Reaction initiative penalties.
pub fn next_personal_round(state: &mut PersonalCombatState) {
    state.round += 1;
    state.active_index = 0;
    for c in &mut state.combatants {
        if c.next_round_init_penalty > 0 {
            c.initiative -= c.next_round_init_penalty;
            c.next_round_init_penalty = 0;
        }
    }
    state.combatants.sort_by_key(|a| std::cmp::Reverse(a.initiative));
    state.log.push(format!("--- Round {} ---", state.round));
}
