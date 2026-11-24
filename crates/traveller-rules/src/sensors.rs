use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::dice::{resolve_task, DiceRollResult, Difficulty, ModifierItem, RollMode};
use crate::ship::SensorGrade;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensorContext {
    SpaceInitialDetection,
    SpaceStealthedReacquire,
    PersonalGround,
}

/// High Guard Apr 2024 pp. 76–79 + CSC 2023 personal armor stealth detection inputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorDetectionInput {
    pub context: SensorContext,
    pub operator_edu_dm: i32,
    pub operator_sensors_skill: i32,
    #[serde(default)]
    pub augment_or_software_dm: i32,
    pub observer_sensor_grade: SensorGrade,
    pub observer_tl: u8,
    pub observer_extended_array: bool,
    pub observer_active_sensors: bool,

    // Target properties
    pub target_tl: u8,
    pub target_active_sensors: bool,
    pub target_thrust_applied: u8,
    pub target_power_plant_active: bool,
    pub target_transponder_or_comms: bool,
    /// Stealth coating / hull / personal armor stealth modifier (e.g. -2, -4, -6).
    pub target_stealth_armor_dm: i32,
    /// Only for stealthed re-acquisition: firing weapons (+2) and crit hit severity (+1/severity).
    #[serde(default)]
    pub target_fired_weapons: bool,
    #[serde(default)]
    pub target_crit_damage_severity: i32,
    /// Extra situational/range/environment DM (e.g. asteroid belt, reflec vs lidar, thermal cloak).
    #[serde(default)]
    pub situational_dm: i32,
    #[serde(default)]
    pub roll_mode: RollMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectionLevel {
    Undetected,
    Minimal,
    Limited,
    Full,
    PinpointLock,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorDetectionResult {
    pub roll: DiceRollResult,
    pub detection_level: DetectionLevel,
    pub observer_signature_exposed: bool,
}

/// Exact High Guard Apr 2024 pp. 76–77 Initial Detection & Stealthed Ship re-acquisition rules.
pub fn resolve_sensor_detection<R: Rng + ?Sized>(
    rng: &mut R,
    input: &SensorDetectionInput,
) -> SensorDetectionResult {
    let mut mods = vec![
        ModifierItem {
            label: "Operator EDU DM".into(),
            value: input.operator_edu_dm,
        },
        ModifierItem {
            label: "Electronics (sensors)".into(),
            value: input.operator_sensors_skill,
        },
        ModifierItem {
            label: format!("Sensor Grade ({:?})", input.observer_sensor_grade),
            value: input.observer_sensor_grade.dm(),
        },
    ];

    if input.augment_or_software_dm != 0 {
        mods.push(ModifierItem {
            label: "Neural Jack / Software".into(),
            value: input.augment_or_software_dm,
        });
    }

    // High Guard p. 76: +1 per higher TL of observer over target
    let tl_diff = i32::from(input.observer_tl) - i32::from(input.target_tl);
    if tl_diff > 0 {
        mods.push(ModifierItem {
            label: format!("TL advantage (TL{} vs TL{})", input.observer_tl, input.target_tl),
            value: tl_diff,
        });
    }

    if input.observer_extended_array {
        mods.push(ModifierItem {
            label: "Extended sensor array deployed".into(),
            value: 2,
        });
    }

    if input.target_active_sensors {
        mods.push(ModifierItem {
            label: "Target active sensors".into(),
            value: 2,
        });
    }

    if input.target_thrust_applied > 0 {
        mods.push(ModifierItem {
            label: format!("Target M-Drive (Thrust {})", input.target_thrust_applied),
            value: i32::from(input.target_thrust_applied),
        });
    }

    if input.target_power_plant_active && input.context == SensorContext::SpaceInitialDetection {
        mods.push(ModifierItem {
            label: "Target power plant active".into(),
            value: 1,
        });
    }

    if input.target_transponder_or_comms {
        mods.push(ModifierItem {
            label: "Target transponder / radio comms".into(),
            value: 6,
        });
    }

    if input.target_stealth_armor_dm != 0 {
        mods.push(ModifierItem {
            label: "Target stealth / armor signature mod".into(),
            value: input.target_stealth_armor_dm,
        });
    }

    if input.target_fired_weapons {
        mods.push(ModifierItem {
            label: "Target fired weapons".into(),
            value: 2,
        });
    }

    if input.target_crit_damage_severity > 0 {
        mods.push(ModifierItem {
            label: format!(
                "Target critical damage heat (Severity {})",
                input.target_crit_damage_severity
            ),
            value: input.target_crit_damage_severity,
        });
    }

    if input.situational_dm != 0 {
        mods.push(ModifierItem {
            label: "Range / environment DM".into(),
            value: input.situational_dm,
        });
    }

    let roll = resolve_task(rng, input.roll_mode, Difficulty::Average, mods);

    let detection_level = if roll.effect <= -6 {
        DetectionLevel::Undetected
    } else if roll.effect < 0 {
        DetectionLevel::Minimal
    } else if roll.effect == 0 {
        DetectionLevel::Limited
    } else if roll.effect < 4 {
        DetectionLevel::Full
    } else {
        DetectionLevel::PinpointLock
    };

    SensorDetectionResult {
        roll,
        detection_level,
        observer_signature_exposed: input.observer_active_sensors,
    }
}
