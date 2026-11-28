use std::collections::BTreeMap;

use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::dice::{resolve_task, roll_nd6, DiceRollResult, Difficulty, ModifierItem, RollMode};

/// Core 2022 p. 166 Space Combat Range Bands & Thrust required to shift by 1 band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpaceRangeBand {
    Adjacent, // 0: 1 Thrust
    Close,    // 1: 1 Thrust
    Short,    // 2: 2 Thrust (+1 Gunner)
    Medium,   // 3: 5 Thrust (0)
    Long,     // 4: 10 Thrust (-2)
    VeryLong, // 5: 25 Thrust (-4)
    Distant,  // 6: 50 Thrust (-6)
}

impl SpaceRangeBand {
    pub fn thrust_to_shift(self) -> u32 {
        match self {
            Self::Adjacent => 1,
            Self::Close => 1,
            Self::Short => 2,
            Self::Medium => 5,
            Self::Long => 10,
            Self::VeryLong => 25,
            Self::Distant => 50,
        }
    }

    pub fn gunner_range_dm(self) -> i32 {
        match self {
            Self::Adjacent | Self::Close | Self::Short => 1,
            Self::Medium => 0,
            Self::Long => -2,
            Self::VeryLong => -4,
            Self::Distant => -6,
        }
    }

    pub fn missile_flight_rounds(self) -> u32 {
        match self {
            Self::Adjacent | Self::Close | Self::Short => 1,
            Self::Medium => 2,
            Self::Long => 3,
            Self::VeryLong => 5,
            Self::Distant => 10,
        }
    }

    pub fn closer(self) -> Self {
        match self {
            Self::Distant => Self::VeryLong,
            Self::VeryLong => Self::Long,
            Self::Long => Self::Medium,
            Self::Medium => Self::Short,
            Self::Short => Self::Close,
            Self::Close | Self::Adjacent => Self::Adjacent,
        }
    }

    pub fn farther(self) -> Self {
        match self {
            Self::Adjacent => Self::Close,
            Self::Close => Self::Short,
            Self::Short => Self::Medium,
            Self::Medium => Self::Long,
            Self::Long => Self::VeryLong,
            Self::VeryLong | Self::Distant => Self::Distant,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpaceCombatPhase {
    Maneuver,
    Attack,
    Actions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CritLocation {
    Sensors,        // 2
    PowerPlant,     // 3
    Fuel,           // 4
    Weapon,         // 5
    Armour,         // 6
    Hull,           // 7
    ManoeuvreDrive, // 8
    Cargo,          // 9
    JumpDrive,      // 10
    Crew,           // 11
    Bridge,         // 12
}

pub fn roll_crit_location<R: Rng + ?Sized>(rng: &mut R) -> CritLocation {
    let d = roll_nd6(rng, 2);
    match d[0] + d[1] {
        2 => CritLocation::Sensors,
        3 => CritLocation::PowerPlant,
        4 => CritLocation::Fuel,
        5 => CritLocation::Weapon,
        6 => CritLocation::Armour,
        7 => CritLocation::Hull,
        8 => CritLocation::ManoeuvreDrive,
        9 => CritLocation::Cargo,
        10 => CritLocation::JumpDrive,
        11 => CritLocation::Crew,
        _ => CritLocation::Bridge,
    }
}

/// Core 2022 p. 170 exact Critical Hit Effects table (Severity 1–6).
pub fn crit_effect_description(loc: CritLocation, severity: u8) -> &'static str {
    let s = severity.clamp(1, 6);
    match (loc, s) {
        (CritLocation::Sensors, 1) => "All sensor checks suffer DM-2",
        (CritLocation::Sensors, 2) => "Sensors inoperative beyond Medium range",
        (CritLocation::Sensors, 3) => "Sensors inoperative beyond Short range",
        (CritLocation::Sensors, 4) => "Sensors inoperative beyond Close range",
        (CritLocation::Sensors, 5) => "Sensors inoperative beyond Adjacent range",
        (CritLocation::Sensors, _) => "Sensors disabled",

        (CritLocation::PowerPlant, 1) => "Power reduced by 10%",
        (CritLocation::PowerPlant, 2) => "Power reduced by 10% (cumulative -20%)",
        (CritLocation::PowerPlant, 3) => "Power reduced by 50%",
        (CritLocation::PowerPlant, 4) => "Power reduced to 0",
        (CritLocation::PowerPlant, 5) => "Hull Severity +1; Power reduced to 0",
        (CritLocation::PowerPlant, _) => "Hull Severity +1D; Power reduced to 0",

        (CritLocation::Fuel, 1) => "Fuel leak: lose 1D tons per hour",
        (CritLocation::Fuel, 2) => "Fuel leak: lose 1D tons per round",
        (CritLocation::Fuel, 3) => "Fuel leak: lose 1D x 10% of fuel",
        (CritLocation::Fuel, 4) => "Fuel tank destroyed",
        (CritLocation::Fuel, 5) => "Fuel tank destroyed; Hull Severity +1",
        (CritLocation::Fuel, _) => "Fuel tank destroyed; Hull Severity +1D",

        (CritLocation::Weapon, 1) => "Random weapon suffers DM-1",
        (CritLocation::Weapon, 2) => "Random weapon disabled",
        (CritLocation::Weapon, 3) => "Random weapon destroyed",
        (CritLocation::Weapon, 4) => "Random weapon explodes; Hull Severity +1",
        (CritLocation::Weapon, 5) => "D3 weapons explode; Hull Severity +1",
        (CritLocation::Weapon, _) => "1D weapons explode; Hull Severity +1",

        (CritLocation::Armour, 1) => "Armour reduced by -1",
        (CritLocation::Armour, 2) => "Armour reduced by -D3",
        (CritLocation::Armour, 3) => "Armour reduced by -1D",
        (CritLocation::Armour, 4) => "Armour reduced by -1D",
        (CritLocation::Armour, _) => "Armour reduced by -2D; Hull Severity +1",

        (CritLocation::Hull, 1) => "Spacecraft suffers 1D extra hull damage",
        (CritLocation::Hull, 2) => "Spacecraft suffers 2D extra hull damage",
        (CritLocation::Hull, 3) => "Spacecraft suffers 3D extra hull damage",
        (CritLocation::Hull, 4) => "Spacecraft suffers 4D extra hull damage",
        (CritLocation::Hull, 5) => "Spacecraft suffers 5D extra hull damage",
        (CritLocation::Hull, _) => "Spacecraft suffers 6D extra hull damage",

        (CritLocation::ManoeuvreDrive, 1) => "Control checks DM-1",
        (CritLocation::ManoeuvreDrive, 2..=4) => "Control checks DM-1 and Thrust reduced by -1",
        (CritLocation::ManoeuvreDrive, 5) => "Thrust reduced to 0",
        (CritLocation::ManoeuvreDrive, _) => "Thrust reduced to 0; Hull Severity +1",

        (CritLocation::Cargo, 1) => "10% of cargo destroyed",
        (CritLocation::Cargo, 2) => "1D x 10% of cargo destroyed",
        (CritLocation::Cargo, 3) => "2D x 10% of cargo destroyed",
        (CritLocation::Cargo, 4) => "All cargo destroyed",
        (CritLocation::Cargo, _) => "All cargo destroyed; Hull Severity +1",

        (CritLocation::JumpDrive, 1) => "Jump checks suffer DM-2",
        (CritLocation::JumpDrive, 2) => "Jump drive disabled",
        (CritLocation::JumpDrive, 3) => "Jump drive destroyed",
        (CritLocation::JumpDrive, _) => "Jump drive destroyed; Hull Severity +1",

        (CritLocation::Crew, 1) => "Random occupant takes 1D damage",
        (CritLocation::Crew, 2) => "Life support fails within 1D hours",
        (CritLocation::Crew, 3) => "1D occupants take 2D damage",
        (CritLocation::Crew, 4) => "Life support fails within 1D rounds",
        (CritLocation::Crew, 5) => "All occupants take 3D damage",
        (CritLocation::Crew, _) => "Life support fails immediately",

        (CritLocation::Bridge, 1) => "Random bridge station disabled",
        (CritLocation::Bridge, 2) => "Computer reboots; software offline this round & next",
        (CritLocation::Bridge, 3) => "Computer damaged; Bandwidth -50%",
        (CritLocation::Bridge, 4) => "Bridge station destroyed; occupant takes 1Dx1D damage",
        (CritLocation::Bridge, 5) => "Computer destroyed",
        (CritLocation::Bridge, _) => "Bridge station destroyed; occupant 1Dx1D; Hull Severity +1",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedCrit {
    pub location: CritLocation,
    pub severity: u8,
    pub effect: String,
    pub extra_hull_damage: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissileSalvo {
    pub id: Uuid,
    pub attacker_ship_id: Uuid,
    pub target_ship_id: Uuid,
    pub count: u32,
    pub smart_tl: u8,
    pub rounds_to_impact: u32,
    pub launched_at_distant: bool,
    pub ew_attempted_this_round: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceCombatShip {
    #[serde(default = "Uuid::new_v4")]
    pub ship_id: Uuid,
    pub name: String,
    pub side: String, // "party" or "hostile"
    #[serde(default = "default_tonnage")]
    pub tonnage: u32,
    #[serde(default = "default_tl")]
    pub tl: u8,
    pub hull_max: i32,
    pub hull_current: i32,
    #[serde(default)]
    pub armor: i32,
    #[serde(default = "default_thrust")]
    pub thrust_max: u8,
    #[serde(default)]
    pub thrust_remaining: u8,
    #[serde(default)]
    pub evasive_dm: i32,
    #[serde(default)]
    pub aid_gunners_dm: i32,
    #[serde(default = "default_power")]
    pub power_available: i32,
    #[serde(default)]
    pub sensor_lock_targets: Vec<Uuid>,
    #[serde(default)]
    pub crits: BTreeMap<CritLocation, u8>,
    #[serde(default)]
    pub initiative: i32,
    #[serde(default)]
    pub last_ten_pct_threshold: i32,
}

fn default_tonnage() -> u32 {
    200
}
fn default_tl() -> u8 {
    12
}
fn default_thrust() -> u8 {
    2
}
fn default_power() -> i32 {
    60
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceCombatState {
    pub id: Uuid,
    pub round: u32,
    pub phase: SpaceCombatPhase,
    pub range_band: SpaceRangeBand,
    pub thrust_progress: i32,
    pub ships: Vec<SpaceCombatShip>,
    pub active_missiles: Vec<MissileSalvo>,
    pub log: Vec<String>,
}

/// Start a Core 2022 / High Guard 2024 6-minute-round spacecraft combat.
pub fn start_space_combat<R: Rng + ?Sized>(
    rng: &mut R,
    range_band: SpaceRangeBand,
    mut ships: Vec<SpaceCombatShip>,
    tactics_naval_party: i32,
    tactics_naval_hostile: i32,
) -> SpaceCombatState {
    for s in &mut ships {
        let d = roll_nd6(rng, 2);
        let t_dm = if s.side == "party" {
            tactics_naval_party
        } else {
            tactics_naval_hostile
        };
        s.initiative = i32::from(d[0]) + i32::from(d[1]) + i32::from(s.thrust_max) + t_dm;
        s.thrust_remaining = s.thrust_max;
        s.last_ten_pct_threshold = 0;
    }
    ships.sort_by_key(|a| std::cmp::Reverse(a.initiative));
    SpaceCombatState {
        id: Uuid::new_v4(),
        round: 1,
        phase: SpaceCombatPhase::Maneuver,
        range_band,
        thrust_progress: 0,
        ships,
        active_missiles: vec![],
        log: vec![format!("Round 1 (Maneuver Step) at {:?} range.", range_band)],
    }
}

/// Allocate Thrust Pool during the Maneuver Step (Range shift, Evasive Action, Aid Gunners).
pub fn allocate_maneuver_thrust<R: Rng + ?Sized>(
    rng: &mut R,
    state: &mut SpaceCombatState,
    ship_id: Uuid,
    thrust_to_range: i32, // negative = close range, positive = open range
    thrust_to_evade: u8,
    pilot_skill: i32,
    aid_gunners: bool,
) -> anyhow::Result<String> {
    let idx = state
        .ships
        .iter()
        .position(|s| s.ship_id == ship_id)
        .ok_or_else(|| anyhow::anyhow!("Ship not found in combat"))?;

    let needed = thrust_to_range.unsigned_abs() as u8
        + thrust_to_evade
        + if aid_gunners { 1 } else { 0 };
    if needed > state.ships[idx].thrust_remaining {
        anyhow::bail!(
            "Requested {needed} Thrust but {} only has {} remaining.",
            state.ships[idx].name,
            state.ships[idx].thrust_remaining
        );
    }
    state.ships[idx].thrust_remaining -= needed;

    // Evasive action: Pilot skill per thrust spent (Core p. 171)
    if thrust_to_evade > 0 {
        state.ships[idx].evasive_dm = i32::from(thrust_to_evade) + pilot_skill.max(0);
    }

    // Aid gunners: Pilot check creating a Task Chain DM
    if aid_gunners {
        let check = resolve_task(
            rng,
            RollMode::Normal,
            Difficulty::Average,
            vec![ModifierItem { label: "Pilot".into(), value: pilot_skill }],
        );
        state.ships[idx].aid_gunners_dm = check.task_chain_dm;
    }

    // Range shift accumulation
    if thrust_to_range != 0 {
        state.thrust_progress += thrust_to_range;
        let threshold = state.range_band.thrust_to_shift() as i32;
        if state.thrust_progress <= -threshold {
            state.range_band = state.range_band.closer();
            state.thrust_progress = 0;
        } else if state.thrust_progress >= threshold {
            state.range_band = state.range_band.farther();
            state.thrust_progress = 0;
        }
    }

    let msg = format!(
        "{} allocated Thrust (range {thrust_to_range:+}, evade {thrust_to_evade} -> DM-{}, aid_gunners DM{:+}). Current range: {:?}.",
        state.ships[idx].name,
        state.ships[idx].evasive_dm,
        state.ships[idx].aid_gunners_dm,
        state.range_band
    );
    state.log.push(msg.clone());
    Ok(msg)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceWeaponAttackInput {
    pub attacker_ship_id: Uuid,
    pub target_ship_id: Uuid,
    pub weapon_name: String, // e.g. "Pulse Laser", "Beam Laser", "Particle Barbette"
    pub linked_count: u8,    // 1 = single, 2 = double turret, 3 = triple turret
    pub damage_dice: u8,
    pub is_laser: bool,
    pub gunner_skill: i32,
    pub gunner_dex_dm: i32,
    pub fire_control_dm: i32,
    pub sandcaster_count_defending: u8,
    pub defender_gunner_skill: i32,
    pub called_shot: Option<CritLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceWeaponAttackResult {
    pub attack_roll: DiceRollResult,
    pub hit: bool,
    pub raw_damage: i32,
    pub sandcaster_negated: i32,
    pub armor_applied: i32,
    pub net_hull_damage: i32,
    pub crits_inflicted: Vec<AppliedCrit>,
    pub target_hull_remaining: i32,
    pub summary: String,
}

fn apply_crit<R: Rng + ?Sized>(
    rng: &mut R,
    ship: &mut SpaceCombatShip,
    loc: CritLocation,
    incoming_severity: u8,
) -> AppliedCrit {
    let prev = ship.crits.get(&loc).copied().unwrap_or(0);
    let mut extra_hull_damage = 0;
    let new_sev = if prev >= 6 {
        // Core p. 170: once at Severity 6, any additional crit deals 6D extra hull damage
        extra_hull_damage = roll_nd6(rng, 6).into_iter().map(i32::from).sum();
        6
    } else {
        let s = incoming_severity.max(prev + 1).min(6);
        ship.crits.insert(loc, s);
        if loc == CritLocation::Hull {
            extra_hull_damage = roll_nd6(rng, usize::from(s)).into_iter().map(i32::from).sum();
        } else if loc == CritLocation::Armour {
            let loss: i32 = match s {
                1 => 1,
                2 => rng.random_range(1..=3),
                3 | 4 => i32::from(roll_nd6(rng, 1)[0]),
                _ => roll_nd6(rng, 2).into_iter().map(i32::from).sum(),
            };
            ship.armor = (ship.armor - loss).max(0);
        } else if loc == CritLocation::ManoeuvreDrive {
            if s >= 5 {
                ship.thrust_max = 0;
            } else if s >= 2 {
                ship.thrust_max = ship.thrust_max.saturating_sub(1);
            }
        }
        s
    };

    if extra_hull_damage > 0 {
        ship.hull_current = (ship.hull_current - extra_hull_damage).max(0);
    }

    AppliedCrit {
        location: loc,
        severity: new_sev,
        effect: crit_effect_description(loc, new_sev).to_string(),
        extra_hull_damage,
    }
}

/// Resolve a direct-fire spacecraft turret/barbette attack (Core 2022 pp. 167–170).
pub fn resolve_space_attack<R: Rng + ?Sized>(
    rng: &mut R,
    state: &mut SpaceCombatState,
    input: &SpaceWeaponAttackInput,
) -> anyhow::Result<SpaceWeaponAttackResult> {
    let atk_idx = state
        .ships
        .iter()
        .position(|s| s.ship_id == input.attacker_ship_id)
        .ok_or_else(|| anyhow::anyhow!("Attacker ship not found"))?;
    let tgt_idx = state
        .ships
        .iter()
        .position(|s| s.ship_id == input.target_ship_id)
        .ok_or_else(|| anyhow::anyhow!("Target ship not found"))?;

    let mut mods = vec![
        ModifierItem { label: "Gunner skill".into(), value: input.gunner_skill },
        ModifierItem { label: "Gunner DEX DM".into(), value: input.gunner_dex_dm },
    ];

    let range_dm = state.range_band.gunner_range_dm();
    if range_dm != 0 {
        mods.push(ModifierItem { label: format!("Range ({:?})", state.range_band), value: range_dm });
    }

    let w_lower = input.weapon_name.to_lowercase();
    if w_lower.contains("beam laser") {
        mods.push(ModifierItem { label: "Beam Laser".into(), value: 4 });
    } else if w_lower.contains("pulse laser") {
        mods.push(ModifierItem { label: "Pulse Laser".into(), value: 2 });
    }

    // Core p. 168: +1 per full 1,000 tons of target (max +6)
    let size_dm = ((state.ships[tgt_idx].tonnage / 1000) as i32).min(6);
    if size_dm > 0 {
        mods.push(ModifierItem { label: "Target size (1000t+)".into(), value: size_dm });
    }

    if state.ships[atk_idx]
        .sensor_lock_targets
        .contains(&input.target_ship_id)
    {
        mods.push(ModifierItem { label: "Sensor Lock".into(), value: 2 });
    }
    if input.fire_control_dm > 0 {
        mods.push(ModifierItem { label: "Fire Control software".into(), value: input.fire_control_dm });
    }
    if state.ships[atk_idx].aid_gunners_dm != 0 {
        mods.push(ModifierItem {
            label: "Pilot Aid Gunners".into(),
            value: state.ships[atk_idx].aid_gunners_dm,
        });
    }
    if state.ships[tgt_idx].evasive_dm > 0 {
        mods.push(ModifierItem {
            label: "Target Evasive Action".into(),
            value: -state.ships[tgt_idx].evasive_dm,
        });
    }
    if input.called_shot.is_some() {
        mods.push(ModifierItem { label: "Called Shot".into(), value: -2 });
    }

    let attack_roll = resolve_task(rng, RollMode::Normal, Difficulty::Average, mods);
    let mut raw_damage = 0;
    let mut sandcaster_negated = 0;
    let mut armor_applied = 0;
    let mut net_hull_damage = 0;
    let mut crits_inflicted = Vec::new();

    if attack_roll.success {
        let dice_sum: i32 = roll_nd6(rng, usize::from(input.damage_dice))
            .into_iter()
            .map(i32::from)
            .sum();
        // Core p. 168: Double/Triple turrets add +1 per damage die for each additional weapon
        let linked_bonus = i32::from(input.linked_count.saturating_sub(1)) * i32::from(input.damage_dice);
        raw_damage = dice_sum + linked_bonus + attack_roll.effect;

        // Core p. 171: Sandcasters add 1D + Effect (plus +1 per linked sandcaster) armor vs lasers
        if input.is_laser && input.sandcaster_count_defending > 0 {
            let sand_check = resolve_task(
                rng,
                RollMode::Normal,
                Difficulty::Average,
                vec![ModifierItem { label: "Defender Gunner".into(), value: input.defender_gunner_skill }],
            );
            if sand_check.success {
                let sand_d = i32::from(roll_nd6(rng, 1)[0]);
                let linked_sand = i32::from(input.sandcaster_count_defending.saturating_sub(1));
                sandcaster_negated = (sand_d + sand_check.effect + linked_sand).max(0);
            }
        }

        let tgt = &mut state.ships[tgt_idx];
        armor_applied = tgt.armor;
        net_hull_damage = (raw_damage - armor_applied - sandcaster_negated).max(0);
        tgt.hull_current = (tgt.hull_current - net_hull_damage).max(0);

        // Critical Hit on Effect 6+ when damage > 0 (Core p. 168–169)
        if net_hull_damage > 0 && attack_roll.effect >= 6 {
            let sev = (attack_roll.effect - 5).clamp(1, 6) as u8;
            let loc = input.called_shot.unwrap_or_else(|| roll_crit_location(rng));
            crits_inflicted.push(apply_crit(rng, tgt, loc, sev));
        }

        // Sustained Damage Critical Hit every 10% of starting Hull lost (Core p. 170)
        if tgt.hull_max > 0 {
            let lost_pct = ((tgt.hull_max - tgt.hull_current) * 100) / tgt.hull_max;
            let thresholds_crossed = (lost_pct / 10) - tgt.last_ten_pct_threshold;
            if thresholds_crossed > 0 {
                tgt.last_ten_pct_threshold = lost_pct / 10;
                for _ in 0..thresholds_crossed {
                    let loc = roll_crit_location(rng);
                    crits_inflicted.push(apply_crit(rng, tgt, loc, 1));
                }
            }
        }
    }

    let tgt_hull = state.ships[tgt_idx].hull_current;
    let atk_name = state.ships[atk_idx].name.clone();
    let tgt_name = state.ships[tgt_idx].name.clone();
    let summary = if attack_roll.success {
        format!(
            "{atk_name} fires {}x {} at {tgt_name} (Effect +{}): {net_hull_damage} hull dmg (Hull {tgt_hull}/{}), {} crit(s).",
            input.linked_count,
            input.weapon_name,
            attack_roll.effect,
            state.ships[tgt_idx].hull_max,
            crits_inflicted.len()
        )
    } else {
        format!(
            "{atk_name} fires {} at {tgt_name} and misses (Effect {}).",
            input.weapon_name, attack_roll.effect
        )
    };
    state.log.push(summary.clone());

    let hit = attack_roll.success;
    Ok(SpaceWeaponAttackResult {
        attack_roll,
        hit,
        raw_damage,
        sandcaster_negated,
        armor_applied,
        net_hull_damage,
        crits_inflicted,
        target_hull_remaining: tgt_hull,
        summary,
    })
}

/// Launch a missile salvo (Core 2022 pp. 173–174), tracked across combat rounds until impact.
pub fn launch_missile_salvo(
    state: &mut SpaceCombatState,
    attacker_ship_id: Uuid,
    target_ship_id: Uuid,
    count: u32,
    smart_tl: u8,
) -> MissileSalvo {
    let salvo = MissileSalvo {
        id: Uuid::new_v4(),
        attacker_ship_id,
        target_ship_id,
        count,
        smart_tl,
        rounds_to_impact: state.range_band.missile_flight_rounds(),
        launched_at_distant: state.range_band == SpaceRangeBand::Distant,
        ew_attempted_this_round: false,
    };
    state.active_missiles.push(salvo.clone());
    state.log.push(format!(
        "Missile salvo ({count} missiles, TL{smart_tl}) launched — impact in {} round(s).",
        salvo.rounds_to_impact
    ));
    salvo
}

/// Electronic Warfare vs. incoming missile salvo (Core p. 173: Difficult 10+ Electronics/sensors
/// removes Effect missiles from the salvo; once per salvo per round).
pub fn ew_jam_missiles<R: Rng + ?Sized>(
    rng: &mut R,
    state: &mut SpaceCombatState,
    salvo_id: Uuid,
    operator_sensors_skill: i32,
    operator_edu_dm: i32,
) -> anyhow::Result<(DiceRollResult, u32)> {
    let salvo = state
        .active_missiles
        .iter_mut()
        .find(|m| m.id == salvo_id)
        .ok_or_else(|| anyhow::anyhow!("Missile salvo not found"))?;
    if salvo.ew_attempted_this_round {
        anyhow::bail!("Electronic Warfare already attempted against this salvo this round.");
    }
    salvo.ew_attempted_this_round = true;

    let roll = resolve_task(
        rng,
        RollMode::Normal,
        Difficulty::Difficult,
        vec![
            ModifierItem { label: "Electronics (sensors)".into(), value: operator_sensors_skill },
            ModifierItem { label: "EDU DM".into(), value: operator_edu_dm },
        ],
    );
    let destroyed = if roll.success {
        let rm = (roll.effect.max(1) as u32).min(salvo.count);
        salvo.count -= rm;
        rm
    } else {
        0
    };
    let rem = salvo.count;
    state.active_missiles.retain(|m| m.count > 0);
    state.log.push(format!(
        "EW missile defense: destroyed {destroyed} missile(s) ({rem} remaining)."
    ));
    Ok((roll, destroyed))
}

/// Core 2022 p. 173: Obtain Sensor Lock (+2 DM on all attacks until broken by EW).
pub fn acquire_sensor_lock<R: Rng + ?Sized>(
    rng: &mut R,
    state: &mut SpaceCombatState,
    ship_id: Uuid,
    target_ship_id: Uuid,
    sensors_skill: i32,
    edu_dm: i32,
) -> anyhow::Result<DiceRollResult> {
    let ship = state
        .ships
        .iter_mut()
        .find(|s| s.ship_id == ship_id)
        .ok_or_else(|| anyhow::anyhow!("Ship not found"))?;
    let roll = resolve_task(
        rng,
        RollMode::Normal,
        Difficulty::Average,
        vec![
            ModifierItem { label: "Electronics (sensors)".into(), value: sensors_skill },
            ModifierItem { label: "EDU DM".into(), value: edu_dm },
        ],
    );
    if roll.success && !ship.sensor_lock_targets.contains(&target_ship_id) {
        ship.sensor_lock_targets.push(target_ship_id);
    }
    Ok(roll)
}

/// Advance Space Combat round: tick incoming missiles (resolving any reaching 0 rounds to impact
/// per Core p. 173: DM +1 per missile + Smart TL - Evade, damage = (4D - Armor) * min(Effect, count)),
/// reset Thrust pools and Evasive/Aid modifiers.
pub fn advance_space_round<R: Rng + ?Sized>(rng: &mut R, state: &mut SpaceCombatState) -> Vec<String> {
    let mut events = Vec::new();
    let mut impacts = Vec::new();
    for m in &mut state.active_missiles {
        m.ew_attempted_this_round = false;
        if m.rounds_to_impact > 0 {
            m.rounds_to_impact -= 1;
        }
        if m.rounds_to_impact == 0 {
            impacts.push(m.clone());
        }
    }
    state.active_missiles.retain(|m| m.rounds_to_impact > 0);

    for salvo in impacts {
        if let Some(tgt) = state.ships.iter_mut().find(|s| s.ship_id == salvo.target_ship_id) {
            let smart_dm = (i32::from(salvo.smart_tl) - 6).clamp(1, 6);
            let mut mods = vec![
                ModifierItem { label: format!("Salvo size ({})", salvo.count), value: salvo.count as i32 },
                ModifierItem { label: format!("Smart TL{}", salvo.smart_tl), value: smart_dm },
            ];
            if salvo.launched_at_distant {
                mods.push(ModifierItem { label: "Distant launch".into(), value: -2 });
            }
            if tgt.evasive_dm > 0 {
                mods.push(ModifierItem { label: "Target Evasive Action".into(), value: -tgt.evasive_dm });
            }
            let roll = resolve_task(rng, RollMode::Normal, Difficulty::Average, mods);
            if roll.success {
                let single_dmg: i32 = roll_nd6(rng, 4).into_iter().map(i32::from).sum();
                let mult = roll.effect.max(1).min(salvo.count as i32);
                let net = ((single_dmg - tgt.armor).max(0)) * mult;
                tgt.hull_current = (tgt.hull_current - net).max(0);
                let msg = format!(
                    "Missile salvo ({}x) IMPACTS {} (Effect +{}): {net} hull damage (Hull {}/{})!",
                    salvo.count, tgt.name, roll.effect, tgt.hull_current, tgt.hull_max
                );
                events.push(msg);
            } else {
                events.push(format!("Missile salvo ({}x) misses {}!", salvo.count, tgt.name));
            }
        }
    }

    state.round += 1;
    state.phase = SpaceCombatPhase::Maneuver;
    for s in &mut state.ships {
        s.thrust_remaining = s.thrust_max;
        s.evasive_dm = 0;
        s.aid_gunners_dm = 0;
    }
    events.push(format!("--- Space Combat Round {} (Maneuver Step) ---", state.round));
    state.log.extend(events.clone());
    events
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceAiActionLog {
    pub ship_id: Uuid,
    pub ship_name: String,
    pub tactical_stance: String,
    pub actions: Vec<String>,
    pub hull_damage_dealt: i32,
    pub crits_inflicted: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceAiTurnReport {
    pub round: u32,
    pub actions: Vec<SpaceAiActionLog>,
    pub summary: String,
}

/// Autonomous High Guard 2024 Tactical AI for hostile ships: evaluates hull integrity,
/// range band, thrust pool, incoming player missiles, sensor locks, and weapons to decide
/// whether to close/evade/flee, jam incoming missiles, acquire sensor lock, fire turrets,
/// or launch missile salvos.
pub fn execute_space_ai_turn<R: Rng + ?Sized>(
    rng: &mut R,
    state: &mut SpaceCombatState,
    player_sandcasters: u8,
    player_gunner_skill: i32,
) -> SpaceAiTurnReport {
    let hostile_ids: Vec<Uuid> = state
        .ships
        .iter()
        .filter(|s| s.side != "party" && s.hull_current > 0)
        .map(|s| s.ship_id)
        .collect();

    let mut report_actions = Vec::new();

    for hid in hostile_ids {
        // Pick primary surviving party target (lowest hull % first to focus fire, else first alive)
        let target_id = match state
            .ships
            .iter()
            .filter(|s| s.side == "party" && s.hull_current > 0)
            .min_by_key(|s| (s.hull_current * 100) / s.hull_max.max(1))
            .map(|s| s.ship_id)
        {
            Some(id) => id,
            None => break,
        };

        let Some(ship) = state.ships.iter().find(|s| s.ship_id == hid).cloned() else {
            continue;
        };
        let hull_pct = (ship.hull_current * 100) / ship.hull_max.max(1);
        let mut ship_actions = Vec::new();
        let mut dmg_dealt = 0;
        let mut crits_dealt = 0;

        let stance = if hull_pct < 30 {
            "evasive_retreat"
        } else if matches!(state.range_band, SpaceRangeBand::VeryLong | SpaceRangeBand::Distant) {
            "closing_engagement"
        } else if hull_pct < 65 {
            "defensive_skirmish"
        } else {
            "aggressive_assault"
        };

        // 1. Maneuver Thrust Pool decision
        if ship.thrust_remaining > 0 {
            let (t_range, t_evade, aid) = match stance {
                "evasive_retreat" => {
                    let ev = (ship.thrust_remaining / 2).max(1).min(ship.thrust_remaining);
                    let rng_t = i32::from(ship.thrust_remaining.saturating_sub(ev));
                    (rng_t, ev, false) // open range (+) & evade
                }
                "closing_engagement" => {
                    let close_t = -(i32::from(ship.thrust_remaining.saturating_sub(1).max(1)));
                    let ev = if ship.thrust_remaining >= 2 { 1 } else { 0 };
                    (close_t, ev, false) // close range (-)
                }
                "defensive_skirmish" => {
                    let ev = (ship.thrust_remaining / 2).max(1);
                    let aid = ship.thrust_remaining > ev;
                    (0, ev, aid)
                }
                _ => {
                    let ev = if ship.thrust_remaining >= 3 { 1 } else { 0 };
                    let aid = ship.thrust_remaining > ev;
                    (0, ev, aid)
                }
            };
            if let Ok(msg) = allocate_maneuver_thrust(rng, state, hid, t_range, t_evade, 1, aid) {
                ship_actions.push(msg);
            }
        }

        // 2. EW Defense vs incoming player missile salvos targeting this ship
        let incoming_salvo = state
            .active_missiles
            .iter()
            .find(|m| m.target_ship_id == hid && !m.ew_attempted_this_round)
            .map(|m| m.id);
        if let Some(salvo_id) = incoming_salvo
            && let Ok((ew_roll, destroyed)) = ew_jam_missiles(rng, state, salvo_id, 1, 1)
        {
            ship_actions.push(format!(
                "EW countermeasure roll {} ({} -> jammed {} missile(s))",
                ew_roll.total,
                if ew_roll.success { "SUCCESS" } else { "FAIL" },
                destroyed
            ));
        }

        // 3. Acquire Sensor Lock if not locked on target
        let already_locked = state
            .ships
            .iter()
            .find(|s| s.ship_id == hid)
            .map(|s| s.sensor_lock_targets.contains(&target_id))
            .unwrap_or(false);
        if !already_locked
            && let Ok(lock_roll) = acquire_sensor_lock(rng, state, hid, target_id, 1, 1)
        {
            ship_actions.push(format!(
                "Sensor Lock attempt: {} ({})",
                lock_roll.total,
                if lock_roll.success { "LOCKED (+2 DM)" } else { "no lock" }
            ));
        }

        // 4. Offensive Weapon Decision: Missiles at Long+ range, or linked Pulse/Beam Laser fire
        let use_missiles = matches!(
            state.range_band,
            SpaceRangeBand::Long | SpaceRangeBand::VeryLong | SpaceRangeBand::Distant
        ) && state.round.is_multiple_of(2);

        if use_missiles {
            let count = if ship.tonnage >= 400 { 4 } else { 2 };
            let salvo = launch_missile_salvo(state, hid, target_id, count, ship.tl);
            ship_actions.push(format!(
                "Launched {count}x Smart TL{} missiles (impact in {} rd(s))",
                ship.tl, salvo.rounds_to_impact
            ));
        } else {
            let linked = if ship.tonnage >= 400 { 3 } else { 2 };
            let weapon = if matches!(state.range_band, SpaceRangeBand::Adjacent | SpaceRangeBand::Close | SpaceRangeBand::Short) {
                "Beam Laser"
            } else {
                "Pulse Laser"
            };
            if let Ok(res) = resolve_space_attack(
                rng,
                state,
                &SpaceWeaponAttackInput {
                    attacker_ship_id: hid,
                    target_ship_id: target_id,
                    weapon_name: weapon.to_string(),
                    linked_count: linked,
                    damage_dice: 2,
                    is_laser: true,
                    gunner_skill: if ship.tonnage >= 400 { 2 } else { 1 },
                    gunner_dex_dm: 1,
                    fire_control_dm: 1,
                    sandcaster_count_defending: player_sandcasters,
                    defender_gunner_skill: player_gunner_skill,
                    called_shot: None,
                },
            ) {
                dmg_dealt += res.net_hull_damage;
                crits_dealt += res.crits_inflicted.len();
                ship_actions.push(res.summary);
            }
        }

        report_actions.push(SpaceAiActionLog {
            ship_id: hid,
            ship_name: ship.name,
            tactical_stance: stance.to_string(),
            actions: ship_actions,
            hull_damage_dealt: dmg_dealt,
            crits_inflicted: crits_dealt,
        });
    }

    let summary = if report_actions.is_empty() {
        "No surviving hostile vessels able to act.".to_string()
    } else {
        let total_dmg: i32 = report_actions.iter().map(|a| a.hull_damage_dealt).sum();
        let total_crits: usize = report_actions.iter().map(|a| a.crits_inflicted).sum();
        format!(
            "Enemy AI executed {} vessel turn(s): {total_dmg} total hull damage, {total_crits} crit(s) inflicted.",
            report_actions.len()
        )
    };

    SpaceAiTurnReport {
        round: state.round,
        actions: report_actions,
        summary,
    }
}
