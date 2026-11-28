use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::dice::roll_nd6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HullConfig {
    Standard,
    Streamlined,
    PartiallyStreamlined,
    Distributed,
    Sphere,
    CloseStructure,
    DispersedStructure,
    Planetoid,
    BufferedPlanetoid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ArmorType {
    None,
    TitaniumSteel,
    #[default]
    Crystaliron,
    BondedSuperdense,
    MolecularBonded,
}

impl ArmorType {
    /// High Guard 2024: percentage of hull tonnage per point of armor, plus min TL.
    pub fn pct_per_point(self) -> f64 {
        match self {
            ArmorType::None => 0.0,
            ArmorType::TitaniumSteel => 2.5,
            ArmorType::Crystaliron => 1.25,
            ArmorType::BondedSuperdense => 0.8,
            ArmorType::MolecularBonded => 0.5,
        }
    }

    pub fn min_tl(self) -> u8 {
        match self {
            ArmorType::None => 0,
            ArmorType::TitaniumSteel => 7,
            ArmorType::Crystaliron => 10,
            ArmorType::BondedSuperdense => 14,
            ArmorType::MolecularBonded => 16,
        }
    }

    pub fn cost_per_point_per_100t_cr(self) -> i64 {
        match self {
            ArmorType::None => 0,
            ArmorType::TitaniumSteel => 250_000,
            ArmorType::Crystaliron => 500_000,
            ArmorType::BondedSuperdense => 750_000,
            ArmorType::MolecularBonded => 1_250_000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensorGrade {
    Basic,    // DM -4
    Civilian, // DM -2
    Military, // DM  0
    Improved, // DM +1
    Advanced, // DM +2
}

impl SensorGrade {
    pub fn dm(self) -> i32 {
        match self {
            SensorGrade::Basic => -4,
            SensorGrade::Civilian => -2,
            SensorGrade::Military => 0,
            SensorGrade::Improved => 1,
            SensorGrade::Advanced => 2,
        }
    }

    pub fn power_required(self) -> i32 {
        match self {
            SensorGrade::Basic => 0,
            SensorGrade::Civilian => 1,
            SensorGrade::Military => 2,
            SensorGrade::Improved => 4,
            SensorGrade::Advanced => 6,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaponMount {
    pub name: String,
    pub mount_kind: String, // single_turret, double_turret, triple_turret, barbette, bay
    pub weapons: Vec<String>,
    pub tonnage: f64,
    pub power: i32,
    pub hardpoints: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipSpec {
    pub tonnage: u32,
    pub tl: u8,
    pub hull_config: HullConfig,
    pub hull_points_max: i32,
    #[serde(default)]
    pub armor_type: ArmorType,
    pub armor_rating: i32,
    pub armor_tonnage: f64,
    pub thrust: u8,
    pub m_drive_tonnage: f64,
    pub jump_rating: u8,
    pub j_drive_tonnage: f64,
    pub power_plant_output: i32,
    pub power_plant_tonnage: f64,
    pub bridge_tonnage: f64,
    #[serde(default = "default_bridge_type")]
    pub bridge_type: String, // "standard", "holographic", "dual_cockpit"
    pub computer_bandwidth: u32,
    #[serde(default)]
    pub computer_bis: bool,
    #[serde(default)]
    pub computer_fib: bool,
    pub sensor_grade: SensorGrade,
    pub sensor_tonnage: f64,
    #[serde(default)]
    pub extended_sensor_array: bool,
    /// Stealth coating DM: 0, -2, -4, or -6 (High Guard 2024 p. 76–77)
    #[serde(default)]
    pub stealth_dm: i32,
    #[serde(default)]
    pub hull_reflec: bool,
    #[serde(default)]
    pub radiation_shielding: bool,
    #[serde(default)]
    pub heat_shielding: bool,
    #[serde(default)]
    pub self_sealing: bool,
    #[serde(default)]
    pub reinforced_hull: bool,
    #[serde(default)]
    pub light_hull: bool,
    pub fuel_capacity_tons: f64,
    #[serde(default)]
    pub fuel_scoops: bool,
    #[serde(default)]
    pub fuel_processor_tons: f64,
    pub staterooms: u32,
    #[serde(default)]
    pub high_staterooms: u32,
    #[serde(default)]
    pub luxury_staterooms: u32,
    #[serde(default)]
    pub barracks_berths: u32,
    pub low_berths: u32,
    #[serde(default)]
    pub emergency_low_berths: u32,
    pub common_areas_tonnage: f64,
    #[serde(default)]
    pub armory_tons: f64,
    #[serde(default)]
    pub medical_bay_tons: f64,
    #[serde(default)]
    pub workshop_tons: f64,
    #[serde(default)]
    pub briefing_room: bool,
    #[serde(default)]
    pub library: bool,
    #[serde(default)]
    pub probe_drones: u32,
    #[serde(default)]
    pub docking_bay_tons: f64,
    #[serde(default)]
    pub cargo_scoops: bool,
    pub cargo_capacity_tons: f64,
    #[serde(default)]
    pub misc_tonnage: f64,
    #[serde(default)]
    pub weapon_mounts: Vec<WeaponMount>,
    pub purchase_price_cr: i64,
}

fn default_bridge_type() -> String {
    "standard".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub severity: String, // "error" or "warning"
    pub category: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipValidationReport {
    pub valid: bool,
    pub used_tonnage: f64,
    pub free_tonnage: f64,
    pub power_basic: i32,
    pub power_m_drive: i32,
    pub power_j_drive: i32,
    pub power_weapons_and_sensors: i32,
    pub power_total_peak_combat: i32,
    pub power_total_jump: i32,
    pub hardpoints_used: u32,
    pub hardpoints_max: u32,
    pub jump_fuel_required: f64,
    pub min_engineers_required: u32,
    pub issues: Vec<ValidationIssue>,
}

/// Exact MgT2e Core 2022 pp. 150–155 & High Guard 2024 ship validation.
pub fn validate_ship(spec: &ShipSpec) -> ShipValidationReport {
    let mut issues = Vec::new();
    let hull = f64::from(spec.tonnage);

    // 1. Tonnage balance (all stateroom/berth types & ship auxiliaries included)
    let weapons_tonnage: f64 = spec.weapon_mounts.iter().map(|w| w.tonnage).sum();
    let stateroom_tonnage = f64::from(spec.staterooms) * 4.0
        + f64::from(spec.high_staterooms) * 6.0
        + f64::from(spec.luxury_staterooms) * 10.0
        + f64::from(spec.barracks_berths) * 2.0
        + f64::from(spec.low_berths) * 0.5
        + f64::from(spec.emergency_low_berths) * 1.0;
    let aux_tonnage = spec.armory_tons
        + spec.medical_bay_tons
        + spec.workshop_tons
        + if spec.briefing_room { 4.0 } else { 0.0 }
        + if spec.library { 4.0 } else { 0.0 }
        + f64::from(spec.probe_drones) * 0.2
        + spec.docking_bay_tons
        + if spec.cargo_scoops { 2.0 } else { 0.0 };
    let used_tonnage = spec.armor_tonnage
        + spec.m_drive_tonnage
        + spec.j_drive_tonnage
        + spec.power_plant_tonnage
        + spec.bridge_tonnage
        + spec.sensor_tonnage
        + spec.fuel_capacity_tons
        + spec.fuel_processor_tons
        + stateroom_tonnage
        + spec.common_areas_tonnage
        + aux_tonnage
        + weapons_tonnage
        + spec.cargo_capacity_tons
        + spec.misc_tonnage;
    let free_tonnage = hull - used_tonnage;

    if free_tonnage < -0.01 {
        issues.push(ValidationIssue {
            severity: "error".into(),
            category: "tonnage".into(),
            message: format!(
                "Ship exceeds hull tonnage by {:.2} tons ({:.2} / {} tons used)",
                -free_tonnage, used_tonnage, spec.tonnage
            ),
        });
    }

    // 2. Hardpoints (1 per 100 tons for ships >= 100t; firmpoints for small craft < 100t)
    let hardpoints_max = if spec.tonnage >= 100 {
        spec.tonnage / 100
    } else {
        (spec.tonnage / 35).max(1)
    };
    let hardpoints_used: u32 = spec.weapon_mounts.iter().map(|w| w.hardpoints).sum();
    if hardpoints_used > hardpoints_max {
        issues.push(ValidationIssue {
            severity: "error".into(),
            category: "hardpoints".into(),
            message: format!(
                "Uses {hardpoints_used} hardpoints/firmpoints, max allowed for {}t hull is {hardpoints_max}",
                spec.tonnage
            ),
        });
    }

    // 3. Power requirements (Core 2022 p. 152–154)
    let power_basic = (hull * 0.20).ceil() as i32;
    let power_m_drive = (hull * 0.10 * f64::from(spec.thrust)).ceil() as i32;
    let power_j_drive = (hull * 0.10 * f64::from(spec.jump_rating)).ceil() as i32;
    let power_weapons: i32 = spec.weapon_mounts.iter().map(|w| w.power).sum();
    let power_weapons_and_sensors = power_weapons + spec.sensor_grade.power_required();
    let power_total_peak_combat = power_basic + power_m_drive + power_weapons_and_sensors;
    let power_total_jump = power_basic + power_j_drive;

    if spec.power_plant_output < power_basic {
        issues.push(ValidationIssue {
            severity: "error".into(),
            category: "power".into(),
            message: format!(
                "Power Plant ({}) cannot even run Basic Ship Systems ({power_basic})",
                spec.power_plant_output
            ),
        });
    } else if spec.power_plant_output < power_total_peak_combat {
        issues.push(ValidationIssue {
            severity: "warning".into(),
            category: "power".into(),
            message: format!(
                "Underpowered for full combat: produces {}, peak combat needs {power_total_peak_combat} (power shifting required)",
                spec.power_plant_output
            ),
        });
    }
    if spec.jump_rating > 0 && spec.power_plant_output < power_total_jump {
        issues.push(ValidationIssue {
            severity: "warning".into(),
            category: "power".into(),
            message: format!(
                "Jump initiation needs {power_total_jump} Power (Basic + J-Drive); plant outputs {} (must reduce basic systems/drives to jump)",
                spec.power_plant_output
            ),
        });
    }

    // 4. Jump fuel check (10% of hull tonnage * Jump rating)
    let jump_fuel_required = hull * 0.10 * f64::from(spec.jump_rating);
    if spec.jump_rating > 0 && spec.fuel_capacity_tons + 0.01 < jump_fuel_required {
        issues.push(ValidationIssue {
            severity: "error".into(),
            category: "fuel".into(),
            message: format!(
                "Jump-{} on {}t hull requires {:.1}t fuel, tank capacity is only {:.1}t",
                spec.jump_rating, spec.tonnage, jump_fuel_required, spec.fuel_capacity_tons
            ),
        });
    }

    // 5. Fuel scoops on unstreamlined hull warning
    if spec.fuel_scoops
        && matches!(
            spec.hull_config,
            HullConfig::Standard
                | HullConfig::Distributed
                | HullConfig::DispersedStructure
                | HullConfig::Planetoid
                | HullConfig::BufferedPlanetoid
        )
    {
        issues.push(ValidationIssue {
            severity: "warning".into(),
            category: "hull".into(),
            message: "Fuel scoops fitted on non-streamlined hull (gas giant skimming hazardous / requires streamlined or partially streamlined hull)".into(),
        });
    }

    // 6. Small craft cannot mount jump drives (Core p. 150)
    if spec.tonnage < 100 && spec.jump_rating > 0 {
        issues.push(ValidationIssue {
            severity: "error".into(),
            category: "jump_drive".into(),
            message: "Small craft (<100 tons) cannot mount a Jump Drive".into(),
        });
    }

    // 7. Minimum engineers (1 per 35 tons of drives + power plant)
    let drive_total_tons = spec.m_drive_tonnage + spec.j_drive_tonnage + spec.power_plant_tonnage;
    let min_engineers_required = if spec.tonnage >= 100 {
        ((drive_total_tons / 35.0).ceil() as u32).max(1)
    } else {
        0
    };

    let valid = !issues.iter().any(|i| i.severity == "error");

    ShipValidationReport {
        valid,
        used_tonnage,
        free_tonnage,
        power_basic,
        power_m_drive,
        power_j_drive,
        power_weapons_and_sensors,
        power_total_peak_combat,
        power_total_jump,
        hardpoints_used,
        hardpoints_max,
        jump_fuel_required,
        min_engineers_required,
        issues,
    }
}

/// Core 2022 p. 150: Ship Mortgage model.
/// Monthly (4-week Maintenance Period) repayment = (Purchase Price - Ship Shares * MCr1) / 240
/// for 40 years (480 periods; or reduced by 10 years / 25% principal per Career Ship Benefit).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mortgage {
    pub active: bool,
    pub original_purchase_price_cr: i64,
    pub ship_shares_applied: u32,
    pub career_benefits_applied: u32,
    pub remortgaged_with_benefits: bool,
    pub monthly_payment_cr: i64,
    pub periods_total: u32,
    pub periods_paid: u32,
    pub periods_overdue: u32,
    pub arrears_cr: i64,
}

impl Mortgage {
    pub fn calculate(
        purchase_price_cr: i64,
        ship_shares: u32,
        career_benefits: u32,
        remortgage_benefits: bool,
    ) -> Self {
        let after_shares = (purchase_price_cr - i64::from(ship_shares) * 1_000_000).max(0);
        let benefit_fraction = (f64::from(career_benefits) * 0.25).min(1.0);

        let (principal, periods_total) = if remortgage_benefits {
            let reduced = ((after_shares as f64) * (1.0 - benefit_fraction)).round() as i64;
            (reduced, 480)
        } else {
            let years_left = (40u32).saturating_sub(career_benefits * 10);
            (after_shares, years_left * 12)
        };

        let monthly_payment_cr = if periods_total == 0 || principal <= 0 {
            0
        } else {
            (principal + 120) / 240 // rounded to nearest Cr
        };

        Self {
            active: monthly_payment_cr > 0 && periods_total > 0,
            original_purchase_price_cr: purchase_price_cr,
            ship_shares_applied: ship_shares,
            career_benefits_applied: career_benefits,
            remortgaged_with_benefits: remortgage_benefits,
            monthly_payment_cr,
            periods_total,
            periods_paid: 0,
            periods_overdue: 0,
            arrears_cr: 0,
        }
    }
}

/// Core 2022 p. 153: Skipping on Debts check when entering a new system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtSkipCheck {
    pub dice: [u8; 2],
    pub total_dm: i32,
    pub total: i32,
    pub hunted: bool,
}

pub fn roll_skipping_on_debts<R: Rng + ?Sized>(
    rng: &mut R,
    parsecs_distant: i32,
    ship_alteration_dm: i32, // -1 to -6
    ship_value_cr: i64,
    visited_recently: bool,
    weeks_overdue: u32,
    local_law_level: i32,
) -> DebtSkipCheck {
    let dice_vec = roll_nd6(rng, 2);
    let dice = [dice_vec[0], dice_vec[1]];
    let overdue_dm = match weeks_overdue {
        0 => return DebtSkipCheck { dice, total_dm: 0, total: 0, hunted: false },
        1..=3 => -4,
        4..=24 => 4,
        25..=52 => 2,
        _ => 0,
    };
    let value_dm = (ship_value_cr / 10_000_000) as i32;
    let recent_dm = if visited_recently { 2 } else { 0 };
    let law_dm = local_law_level - 5;
    let total_dm =
        -parsecs_distant + ship_alteration_dm.clamp(-6, 0) + value_dm + recent_dm + overdue_dm + law_dm;
    let total = i32::from(dice[0]) + i32::from(dice[1]) + total_dm;
    DebtSkipCheck {
        dice,
        total_dm,
        total,
        hunted: total >= 8,
    }
}

/// Core 2022 p. 154: Maintenance Period (every 4 weeks) running costs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningCostsBreakdown {
    pub mortgage_cr: i64,
    pub maintenance_cr: i64, // 0.1% of purchase price / 12
    pub life_support_cr: i64,
    pub crew_salaries_cr: i64,
    pub total_cr: i64,
}

pub fn maintenance_period_cost(purchase_price_cr: i64) -> i64 {
    ((purchase_price_cr as f64 * 0.001) / 12.0).round() as i64
}

pub fn life_support_cost(staterooms: u32, non_low_berth_occupants: u32, occupied_low_berths: u32) -> i64 {
    i64::from(staterooms) * 1_000
        + i64::from(non_low_berth_occupants) * 1_000
        + i64::from(occupied_low_berths) * 100
}

pub fn compute_running_costs(
    spec: &ShipSpec,
    mortgage: Option<&Mortgage>,
    non_low_berth_occupants: u32,
    occupied_low_berths: u32,
    crew_salaries_cr: i64,
) -> RunningCostsBreakdown {
    let mortgage_cr = mortgage
        .filter(|m| m.active && m.periods_paid < m.periods_total)
        .map(|m| m.monthly_payment_cr)
        .unwrap_or(0);
    let maintenance_cr = maintenance_period_cost(spec.purchase_price_cr);
    let life_support_cr =
        life_support_cost(spec.staterooms, non_low_berth_occupants, occupied_low_berths);
    let total_cr = mortgage_cr + maintenance_cr + life_support_cr + crew_salaries_cr;
    RunningCostsBreakdown {
        mortgage_cr,
        maintenance_cr,
        life_support_cr,
        crew_salaries_cr,
        total_cr,
    }
}

/// Core 2022 p. 154: Poor Maintenance Critical Hit table when maintenance is skipped.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoorMaintenanceResult {
    pub trigger_roll: i32,
    pub skipped_periods: u32,
    pub suffered_crit: bool,
    pub crit_table_roll: Option<i32>,
    pub crit_name: Option<String>,
    pub effect: Option<String>,
}

pub fn roll_poor_maintenance<R: Rng + ?Sized>(
    rng: &mut R,
    skipped_periods: u32,
) -> PoorMaintenanceResult {
    if skipped_periods == 0 {
        return PoorMaintenanceResult {
            trigger_roll: 0,
            skipped_periods: 0,
            suffered_crit: false,
            crit_table_roll: None,
            crit_name: None,
            effect: None,
        };
    }
    let trigger_dice = roll_nd6(rng, 2);
    let trigger_roll =
        i32::from(trigger_dice[0]) + i32::from(trigger_dice[1]) + skipped_periods as i32;
    if trigger_roll < 8 {
        return PoorMaintenanceResult {
            trigger_roll,
            skipped_periods,
            suffered_crit: false,
            crit_table_roll: None,
            crit_name: None,
            effect: None,
        };
    }
    let crit_dice = roll_nd6(rng, 2);
    let crit_roll = i32::from(crit_dice[0]) + i32::from(crit_dice[1]);
    let (name, effect) = match crit_roll {
        2..=4 => {
            let pct = i32::from(roll_nd6(rng, 1)[0]) * 10;
            (
                "Fuel Leak",
                format!("Spacecraft loses {pct}% of its fuel capacity."),
            )
        }
        5..=7 => {
            let d = roll_nd6(rng, 1)[0];
            if d <= 3 {
                (
                    "Drive Damaged (M-Drive)",
                    "Manoeuvre drive hit: Thrust -1 and all Pilot checks suffer DM-1.".to_string(),
                )
            } else {
                (
                    "Drive Damaged (J-Drive)",
                    "Jump drive hit and disabled until repaired.".to_string(),
                )
            }
        }
        8..=9 => (
            "Weapon Faulty",
            "One random turret/weapon suffers DM-1 on all attack rolls until repaired.".to_string(),
        ),
        _ => {
            let hull_dmg = roll_nd6(rng, 1)[0];
            let rad_dice = roll_nd6(rng, 2);
            let rads = (i32::from(rad_dice[0]) + i32::from(rad_dice[1])) * 10;
            (
                "Power Plant",
                format!(
                    "Power reduced by 25%, takes {hull_dmg} hull damage (ignoring armour), crew suffers {rads} rads/week."
                ),
            )
        }
    };

    PoorMaintenanceResult {
        trigger_roll,
        skipped_periods,
        suffered_crit: true,
        crit_table_roll: Some(crit_roll),
        crit_name: Some(name.to_string()),
        effect: Some(effect),
    }
}

/// Core 2022: A fleet jumping together is limited to the LOWEST Jump Rating among its ships.
pub fn fleet_max_jump_rating(specs: &[&ShipSpec]) -> u8 {
    specs.iter().map(|s| s.jump_rating).min().unwrap_or(0)
}

/// Core 2022 p. 151: Jump fuel consumed = 10% of hull tonnage per parsec jumped.
pub fn jump_fuel_for_distance(tonnage: u32, parsecs: u8) -> f64 {
    f64::from(tonnage) * 0.10 * f64::from(parsecs.max(1))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefuelMode {
    Refined,
    Unrefined,
    GasGiantSkim,
    ProcessOnboard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefuelQuote {
    pub allowed: bool,
    pub mode: RefuelMode,
    pub tons: f64,
    pub cost_per_ton_cr: i64,
    pub total_cost_cr: i64,
    pub hours_required: u32,
    pub note: String,
}

/// Core 2022 pp. 151–152 Refueling costs & requirements (Refined Cr500/t at A/B, Unrefined Cr100/t at A–D,
/// Gas Giant skimming Cr0 with scoops, Onboard Fuel Processor 20t/day per processor ton).
pub fn calculate_refuel_quote(
    starport: char,
    gas_giants: i16,
    spec: &ShipSpec,
    requested_tons: f64,
    mode: RefuelMode,
) -> RefuelQuote {
    let tons = requested_tons.max(0.0);
    match mode {
        RefuelMode::Refined => {
            if !matches!(starport, 'A' | 'B') {
                return RefuelQuote {
                    allowed: false,
                    mode,
                    tons: 0.0,
                    cost_per_ton_cr: 500,
                    total_cost_cr: 0,
                    hours_required: 0,
                    note: format!("Starport Class {starport} does not offer Refined fuel (requires Class A or B)."),
                };
            }
            let total = (tons * 500.0).round() as i64;
            RefuelQuote {
                allowed: true,
                mode,
                tons,
                cost_per_ton_cr: 500,
                total_cost_cr: total,
                hours_required: 1,
                note: format!("Purchased {tons:.1}t Refined fuel at Starport {starport} for Cr{total}."),
            }
        }
        RefuelMode::Unrefined => {
            if !matches!(starport, 'A' | 'B' | 'C' | 'D') {
                return RefuelQuote {
                    allowed: false,
                    mode,
                    tons: 0.0,
                    cost_per_ton_cr: 100,
                    total_cost_cr: 0,
                    hours_required: 0,
                    note: format!("Starport Class {starport} has no commercial fuel facilities (try Gas Giant skimming)."),
                };
            }
            let total = (tons * 100.0).round() as i64;
            RefuelQuote {
                allowed: true,
                mode,
                tons,
                cost_per_ton_cr: 100,
                total_cost_cr: total,
                hours_required: 1,
                note: format!("Purchased {tons:.1}t Unrefined fuel at Starport {starport} for Cr{total} (DM-2 Astrogation / misjump risk unless processed)."),
            }
        }
        RefuelMode::GasGiantSkim => {
            if gas_giants <= 0 {
                return RefuelQuote {
                    allowed: false,
                    mode,
                    tons: 0.0,
                    cost_per_ton_cr: 0,
                    total_cost_cr: 0,
                    hours_required: 0,
                    note: "No gas giant in this system to skim fuel from.".into(),
                };
            }
            if !spec.fuel_scoops {
                return RefuelQuote {
                    allowed: false,
                    mode,
                    tons: 0.0,
                    cost_per_ton_cr: 0,
                    total_cost_cr: 0,
                    hours_required: 0,
                    note: "Ship does not have Fuel Scoops installed for gas giant skimming.".into(),
                };
            }
            RefuelQuote {
                allowed: true,
                mode,
                tons,
                cost_per_ton_cr: 0,
                total_cost_cr: 0,
                hours_required: 6,
                note: format!("Skimmed {tons:.1}t Unrefined fuel from gas giant (Cr0; 6h transit/skimming)."),
            }
        }
        RefuelMode::ProcessOnboard => {
            if spec.fuel_processor_tons <= 0.0 {
                return RefuelQuote {
                    allowed: false,
                    mode,
                    tons: 0.0,
                    cost_per_ton_cr: 0,
                    total_cost_cr: 0,
                    hours_required: 0,
                    note: "Ship does not have an onboard Fuel Processor installed.".into(),
                };
            }
            let tons_per_hour = (spec.fuel_processor_tons * 20.0) / 24.0;
            let hours = (tons / tons_per_hour).ceil().max(1.0) as u32;
            RefuelQuote {
                allowed: true,
                mode,
                tons,
                cost_per_ton_cr: 0,
                total_cost_cr: 0,
                hours_required: hours,
                note: format!("Refined {tons:.1}t unrefined fuel onboard in {hours}h."),
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BerthingFeeResult {
    pub starport: char,
    pub roll_1d6: u8,
    pub base_berthing_cr: i64,
    pub extra_days: u32,
    pub extra_days_cr: i64,
    pub total_cr: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccruedBerthingFee {
    pub starport: char,
    pub total_days_docked: u32,
    pub newly_billable_days: u32,
    pub daily_rate_cr: i64,
    pub fee_cr: i64,
}

/// Core 2022 p. 235 daily extended-berthing rate after the initial 6 prepaid days.
pub fn berthing_extra_day_rate(starport: char) -> i64 {
    match starport.to_ascii_uppercase() {
        'A' => 500,
        'B' => 200,
        'C' => 100,
        'D' => 10,
        _ => 0,
    }
}

/// Core 2022 p. 235 Starport Berthing / Docking Fees (covers 1–6 days + extra days).
pub fn roll_berthing_fee<R: Rng + ?Sized>(
    rng: &mut R,
    starport: char,
    extra_days: u32,
) -> BerthingFeeResult {
    let d = roll_nd6(rng, 1)[0];
    let mult: i64 = match starport.to_ascii_uppercase() {
        'A' => 1_000,
        'B' => 500,
        'C' => 100,
        'D' => 10,
        _ => 0,
    };
    let per_extra_day = berthing_extra_day_rate(starport);
    let base_berthing_cr = i64::from(d) * mult;
    let extra_days_cr = i64::from(extra_days) * per_extra_day;
    BerthingFeeResult {
        starport,
        roll_1d6: d,
        base_berthing_cr,
        extra_days,
        extra_days_cr,
        total_cr: base_berthing_cr + extra_days_cr,
    }
}

/// Computes newly billable extended-stay berthing fees as campaign time advances
/// while a ship remains docked at a starport. The initial docking fee covers the
/// first 6 days (`already_billed_days` starts at 6 upon docking); each full day
/// beyond that accrues the starport's daily extended-berthing rate.
pub fn calculate_accrued_berthing_fee(
    starport: char,
    docked_at_total_minutes: i64,
    now_total_minutes: i64,
    already_billed_days: u32,
) -> AccruedBerthingFee {
    let elapsed_mins = (now_total_minutes - docked_at_total_minutes).max(0);
    let total_days_docked = (elapsed_mins / 1440) as u32;
    let covered = already_billed_days.max(6);
    let newly_billable_days = total_days_docked.saturating_sub(covered);
    let daily_rate_cr = berthing_extra_day_rate(starport);
    let fee_cr = i64::from(newly_billable_days) * daily_rate_cr;
    AccruedBerthingFee {
        starport,
        total_days_docked,
        newly_billable_days,
        daily_rate_cr,
        fee_cr,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefitLineItem {
    pub item: String,
    pub cost_cr: i64,
    pub requires_shipyard: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefitQuote {
    pub allowed: bool,
    pub block_reason: Option<String>,
    pub has_changes: bool,
    pub total_cost_cr: i64,
    pub days_required: u32,
    pub items: Vec<RefitLineItem>,
}

fn sensor_base_cost_cr(g: SensorGrade) -> i64 {
    match g {
        SensorGrade::Basic => 0,
        SensorGrade::Civilian => 3_000_000,
        SensorGrade::Military => 4_100_000,
        SensorGrade::Improved => 4_300_000,
        SensorGrade::Advanced => 5_300_000,
    }
}

fn stealth_cost_cr(tonnage: u32, stealth_dm: i32) -> i64 {
    let t = i64::from(tonnage);
    match stealth_dm {
        d if d <= -6 => t * 500_000,
        d if d <= -4 => t * 200_000,
        d if d <= -2 => t * 100_000,
        _ => 0,
    }
}

/// Core 2022 / High Guard 2024 Shipyard Refit & Repair Cost Calculator.
/// Prevents free ship alterations: calculates exact itemized Cr costs, enforces Starport A/B shipyard
/// requirements for major refits (Starport C for minor hull repair), and checks world Tech Level.
#[allow(clippy::too_many_arguments)]
pub fn calculate_refit_cost(
    current: &ShipSpec,
    current_hull: i32,
    current_crits: usize,
    proposed: &ShipSpec,
    proposed_hull: i32,
    proposed_crits: usize,
    starport: char,
    world_tl: u8,
) -> RefitQuote {
    let mut items = Vec::new();
    let mut needs_shipyard = false;
    let mut needs_repair_yard = false;
    let mut days = 0u32;

    if proposed.tonnage != current.tonnage {
        return RefitQuote {
            allowed: false,
            block_reason: Some("Hull tonnage cannot be changed via refit — buy or build a new hull at a Class A shipyard.".into()),
            has_changes: true,
            total_cost_cr: 0,
            days_required: 0,
            items,
        };
    }

    if proposed.tl > current.tl && proposed.tl > world_tl {
        return RefitQuote {
            allowed: false,
            block_reason: Some(format!(
                "Cannot upgrade ship to TL{} here (world is only TL{}).",
                proposed.tl, world_tl
            )),
            has_changes: true,
            total_cost_cr: 0,
            days_required: 0,
            items,
        };
    }

    // 1. Hull Damage Repair (Core 2022 p. 175: Cr100,000 per Hull point at shipyard)
    if proposed_hull > current_hull {
        let pts = i64::from(proposed_hull - current_hull);
        let cost = pts * 100_000;
        items.push(RefitLineItem {
            item: format!("Hull repair (+{pts} Hull points)"),
            cost_cr: cost,
            requires_shipyard: false,
        });
        needs_repair_yard = true;
        days += ((pts as u32) / 10).max(1);
    }

    // 2. Critical Hit Repairs (Cr250,000 per crit)
    if proposed_crits < current_crits {
        let fixed = (current_crits - proposed_crits) as i64;
        items.push(RefitLineItem {
            item: format!("Critical system repair ({fixed} crit(s))"),
            cost_cr: fixed * 250_000,
            requires_shipyard: false,
        });
        needs_repair_yard = true;
        days += fixed as u32 * 2;
    }

    // 3. Hull Max / Structural Upgrade
    if proposed.hull_points_max > current.hull_points_max {
        let delta = i64::from(proposed.hull_points_max - current.hull_points_max);
        items.push(RefitLineItem {
            item: format!("Reinforced hull structure (+{delta} max Hull)"),
            cost_cr: delta * 150_000,
            requires_shipyard: true,
        });
        needs_shipyard = true;
        days += 14;
    }

    // 4. Armor Rating / Type Upgrade (High Guard 2024)
    if proposed.armor_rating != current.armor_rating || proposed.armor_type != current.armor_type {
        let units = i64::from((proposed.tonnage / 100).max(1));
        let old_val = i64::from(current.armor_rating) * current.armor_type.cost_per_point_per_100t_cr() * units;
        let new_val = i64::from(proposed.armor_rating) * proposed.armor_type.cost_per_point_per_100t_cr() * units;
        let cost = (new_val - old_val / 2).max(100_000);
        items.push(RefitLineItem {
            item: format!(
                "Armor refit ({:?} {} → {:?} {})",
                current.armor_type, current.armor_rating, proposed.armor_type, proposed.armor_rating
            ),
            cost_cr: cost,
            requires_shipyard: true,
        });
        needs_shipyard = true;
        days += 7;
    }

    // 4b. Hull Configuration & Hull Features (Reflec, Radiation/Heat Shielding, Self-Sealing, Reinforced/Light)
    if proposed.hull_config != current.hull_config {
        let cost = i64::from(proposed.tonnage) * 15_000;
        items.push(RefitLineItem {
            item: format!("Hull reconfiguration ({:?} → {:?})", current.hull_config, proposed.hull_config),
            cost_cr: cost,
            requires_shipyard: true,
        });
        needs_shipyard = true;
        days += 28;
    }
    if proposed.hull_reflec != current.hull_reflec {
        let cost = if proposed.hull_reflec { i64::from(proposed.tonnage) * 100_000 } else { 100_000 };
        items.push(RefitLineItem {
            item: if proposed.hull_reflec { "Install Reflec Hull Coating (+3 vs Lasers)".into() } else { "Strip Reflec Coating".into() },
            cost_cr: cost,
            requires_shipyard: true,
        });
        needs_shipyard = true;
        days += 7;
    }
    if proposed.radiation_shielding != current.radiation_shielding {
        let cost = if proposed.radiation_shielding { i64::from(proposed.tonnage) * 25_000 } else { 50_000 };
        items.push(RefitLineItem {
            item: if proposed.radiation_shielding { "Install Radiation Shielding".into() } else { "Remove Radiation Shielding".into() },
            cost_cr: cost,
            requires_shipyard: true,
        });
        needs_shipyard = true;
        days += 5;
    }
    if proposed.heat_shielding != current.heat_shielding {
        let cost = if proposed.heat_shielding { i64::from(proposed.tonnage) * 100_000 } else { 50_000 };
        items.push(RefitLineItem {
            item: if proposed.heat_shielding { "Install Heat Shielding (Corona/Entry)".into() } else { "Remove Heat Shielding".into() },
            cost_cr: cost,
            requires_shipyard: true,
        });
        needs_shipyard = true;
        days += 5;
    }
    if proposed.self_sealing != current.self_sealing {
        let cost = if proposed.self_sealing { i64::from(proposed.tonnage) * 10_000 } else { 25_000 };
        items.push(RefitLineItem {
            item: if proposed.self_sealing { "Install Self-Sealing Hull".into() } else { "Remove Self-Sealing Hull".into() },
            cost_cr: cost,
            requires_shipyard: true,
        });
        needs_shipyard = true;
        days += 4;
    }
    if proposed.reinforced_hull != current.reinforced_hull {
        let cost = if proposed.reinforced_hull { i64::from(proposed.tonnage) * 25_000 } else { 100_000 };
        items.push(RefitLineItem {
            item: if proposed.reinforced_hull { "Reinforced Hull Structural Bracing (+10% Hull)".into() } else { "Remove Reinforced Hull Bracing".into() },
            cost_cr: cost,
            requires_shipyard: true,
        });
        needs_shipyard = true;
        days += 14;
    }
    if proposed.light_hull != current.light_hull {
        items.push(RefitLineItem {
            item: if proposed.light_hull { "Lightened Hull Conversion (-10% Hull)".into() } else { "Restore Standard Hull Framing".into() },
            cost_cr: i64::from(proposed.tonnage) * 10_000,
            requires_shipyard: true,
        });
        needs_shipyard = true;
        days += 14;
    }

    // 5. M-Drive Thrust Refit (Core/HG: MCr2 per drive ton, ~1% hull per thrust)
    if proposed.thrust != current.thrust {
        let delta = i64::from(proposed.thrust.abs_diff(current.thrust));
        let drive_tons = (f64::from(proposed.tonnage) * 0.01 * delta as f64).max(1.0);
        let cost = if proposed.thrust > current.thrust {
            (drive_tons * 2_000_000.0).round() as i64
        } else {
            250_000
        };
        items.push(RefitLineItem {
            item: format!("M-Drive refit (M-{} → M-{})", current.thrust, proposed.thrust),
            cost_cr: cost,
            requires_shipyard: true,
        });
        needs_shipyard = true;
        days += 14;
    }

    // 6. J-Drive Jump Rating Refit (Core/HG: MCr1.5 per drive ton, ~2.5% hull per jump + 5t)
    if proposed.jump_rating != current.jump_rating {
        let delta = i64::from(proposed.jump_rating.abs_diff(current.jump_rating));
        let drive_tons = (f64::from(proposed.tonnage) * 0.025 * delta as f64 + 5.0).max(5.0);
        let cost = if proposed.jump_rating > current.jump_rating {
            (drive_tons * 1_500_000.0).round() as i64
        } else {
            500_000
        };
        items.push(RefitLineItem {
            item: format!("J-Drive refit (J-{} → J-{})", current.jump_rating, proposed.jump_rating),
            cost_cr: cost,
            requires_shipyard: true,
        });
        needs_shipyard = true;
        days += 21;
    }

    // 7. Power Plant Refit
    if proposed.power_plant_output != current.power_plant_output {
        let delta = i64::from((proposed.power_plant_output - current.power_plant_output).abs());
        let cost = if proposed.power_plant_output > current.power_plant_output {
            delta * 100_000
        } else {
            100_000
        };
        items.push(RefitLineItem {
            item: format!(
                "Power Plant refit ({} → {} Power)",
                current.power_plant_output, proposed.power_plant_output
            ),
            cost_cr: cost,
            requires_shipyard: true,
        });
        needs_shipyard = true;
        days += 10;
    }

    // 8. Sensor Suite Upgrade
    if proposed.sensor_grade != current.sensor_grade {
        let old_c = sensor_base_cost_cr(current.sensor_grade);
        let new_c = sensor_base_cost_cr(proposed.sensor_grade);
        let cost = (new_c - old_c / 2).max(250_000);
        items.push(RefitLineItem {
            item: format!("Sensor suite refit ({:?} → {:?})", current.sensor_grade, proposed.sensor_grade),
            cost_cr: cost,
            requires_shipyard: true,
        });
        needs_shipyard = true;
        days += 5;
    }

    // 9. Stealth Coating (High Guard 2024)
    if proposed.stealth_dm != current.stealth_dm {
        let old_c = stealth_cost_cr(current.tonnage, current.stealth_dm);
        let new_c = stealth_cost_cr(proposed.tonnage, proposed.stealth_dm);
        let cost = (new_c - old_c).max(100_000);
        items.push(RefitLineItem {
            item: format!("Stealth coating refit (DM {} → DM {})", current.stealth_dm, proposed.stealth_dm),
            cost_cr: cost,
            requires_shipyard: true,
        });
        needs_shipyard = true;
        days += 7;
    }

    // 10. All Stateroom, Berth, Bridge, Computer, Fuel & Auxiliary Reconfiguration
    let mut add_count_refit = |label: &str, old_v: u32, new_v: u32, unit_buy_cr: i64, unit_rm_cr: i64, d: u32| {
        if new_v != old_v {
            let delta = i64::from(new_v.abs_diff(old_v));
            let cost = if new_v > old_v { delta * unit_buy_cr } else { delta * unit_rm_cr };
            items.push(RefitLineItem {
                item: format!("{label} ({old_v} → {new_v})"),
                cost_cr: cost,
                requires_shipyard: true,
            });
            needs_shipyard = true;
            days += d;
        }
    };
    add_count_refit("Standard Staterooms (4t)", current.staterooms, proposed.staterooms, 500_000, 50_000, 4);
    add_count_refit("High Staterooms (6t)", current.high_staterooms, proposed.high_staterooms, 800_000, 80_000, 5);
    add_count_refit("Luxury Staterooms (10t)", current.luxury_staterooms, proposed.luxury_staterooms, 1_500_000, 100_000, 7);
    add_count_refit("Marine Barracks (2t)", current.barracks_berths, proposed.barracks_berths, 100_000, 20_000, 3);
    add_count_refit("Low Berths (0.5t)", current.low_berths, proposed.low_berths, 50_000, 10_000, 2);
    add_count_refit("Emergency Low Berths (1t)", current.emergency_low_berths, proposed.emergency_low_berths, 100_000, 15_000, 2);
    add_count_refit("Probe Drones (0.2t)", current.probe_drones, proposed.probe_drones, 100_000, 5_000, 1);

    if (proposed.common_areas_tonnage - current.common_areas_tonnage).abs() > 0.1 {
        let delta = (proposed.common_areas_tonnage - current.common_areas_tonnage).abs();
        let cost = if proposed.common_areas_tonnage > current.common_areas_tonnage {
            (delta * 100_000.0).round() as i64
        } else {
            (delta * 15_000.0).round() as i64
        };
        items.push(RefitLineItem {
            item: format!("Common Areas ({:.1}t → {:.1}t)", current.common_areas_tonnage, proposed.common_areas_tonnage),
            cost_cr: cost,
            requires_shipyard: true,
        });
        needs_shipyard = true;
        days += 2;
    }

    if proposed.bridge_type != current.bridge_type {
        let cost = match proposed.bridge_type.as_str() {
            "holographic" => 250_000 * i64::from((proposed.tonnage / 100).max(1)),
            _ => 150_000,
        };
        items.push(RefitLineItem {
            item: format!("Bridge controls ({} → {})", current.bridge_type, proposed.bridge_type),
            cost_cr: cost,
            requires_shipyard: true,
        });
        needs_shipyard = true;
        days += 5;
    }

    if proposed.computer_bandwidth != current.computer_bandwidth
        || proposed.computer_bis != current.computer_bis
        || proposed.computer_fib != current.computer_fib
    {
        let comp_cost = |bw: u32, bis: bool, fib: bool| -> i64 {
            let base: i64 = match bw {
                0..=5 => 30_000,
                6..=10 => 160_000,
                11..=15 => 2_000_000,
                16..=20 => 5_000_000,
                21..=25 => 10_000_000,
                26..=30 => 20_000_000,
                _ => 30_000_000,
            };
            let mult = 1.0 + if bis { 0.5 } else { 0.0 } + if fib { 0.5 } else { 0.0 };
            (base as f64 * mult).round() as i64
        };
        let old_c = comp_cost(current.computer_bandwidth, current.computer_bis, current.computer_fib);
        let new_c = comp_cost(proposed.computer_bandwidth, proposed.computer_bis, proposed.computer_fib);
        items.push(RefitLineItem {
            item: format!(
                "Ship Computer (/{} → /{}{}{})",
                current.computer_bandwidth,
                proposed.computer_bandwidth,
                if proposed.computer_bis { "bis" } else { "" },
                if proposed.computer_fib { "fib" } else { "" }
            ),
            cost_cr: (new_c - old_c / 2).max(50_000),
            requires_shipyard: true,
        });
        needs_shipyard = true;
        days += 3;
    }

    if proposed.fuel_scoops != current.fuel_scoops {
        items.push(RefitLineItem {
            item: if proposed.fuel_scoops { "Install Fuel Scoops".into() } else { "Remove Fuel Scoops".into() },
            cost_cr: if proposed.fuel_scoops { 1_000_000 } else { 50_000 },
            requires_shipyard: true,
        });
        needs_shipyard = true;
        days += 3;
    }
    if (proposed.fuel_processor_tons - current.fuel_processor_tons).abs() > 0.1 {
        let delta = (proposed.fuel_processor_tons - current.fuel_processor_tons).abs();
        items.push(RefitLineItem {
            item: format!("Fuel Processor ({:.1}t → {:.1}t)", current.fuel_processor_tons, proposed.fuel_processor_tons),
            cost_cr: if proposed.fuel_processor_tons > current.fuel_processor_tons {
                (delta * 50_000.0).round() as i64
            } else {
                25_000
            },
            requires_shipyard: true,
        });
        needs_shipyard = true;
        days += 2;
    }

    for (label, old_t, new_t, rate) in [
        ("Ship's Armory", current.armory_tons, proposed.armory_tons, 250_000.0),
        ("Medical Bay", current.medical_bay_tons, proposed.medical_bay_tons, 500_000.0),
        ("Workshop", current.workshop_tons, proposed.workshop_tons, 150_000.0),
        ("Small Craft Docking Bay", current.docking_bay_tons, proposed.docking_bay_tons, 200_000.0),
    ] {
        if (new_t - old_t).abs() > 0.1 {
            let delta = (new_t - old_t).abs();
            items.push(RefitLineItem {
                item: format!("{label} ({old_t:.1}t → {new_t:.1}t)"),
                cost_cr: if new_t > old_t { (delta * rate).round() as i64 } else { (delta * 20_000.0).round() as i64 },
                requires_shipyard: true,
            });
            needs_shipyard = true;
            days += 3;
        }
    }
    if proposed.briefing_room != current.briefing_room {
        items.push(RefitLineItem {
            item: if proposed.briefing_room { "Install Briefing Room (4t)".into() } else { "Remove Briefing Room".into() },
            cost_cr: if proposed.briefing_room { 500_000 } else { 50_000 },
            requires_shipyard: true,
        });
        needs_shipyard = true;
        days += 2;
    }
    if proposed.library != current.library {
        items.push(RefitLineItem {
            item: if proposed.library { "Install Ship's Library (4t)".into() } else { "Remove Ship's Library".into() },
            cost_cr: if proposed.library { 4_000_000 } else { 50_000 },
            requires_shipyard: true,
        });
        needs_shipyard = true;
        days += 2;
    }
    if proposed.cargo_scoops != current.cargo_scoops {
        items.push(RefitLineItem {
            item: if proposed.cargo_scoops { "Install Cargo Scoop (2t)".into() } else { "Remove Cargo Scoop".into() },
            cost_cr: if proposed.cargo_scoops { 500_000 } else { 50_000 },
            requires_shipyard: true,
        });
        needs_shipyard = true;
        days += 2;
    }

    // 11. Weapon Mounts / Turrets / Barbettes Refit
    let old_w_json = serde_json::to_string(&current.weapon_mounts).unwrap_or_default();
    let new_w_json = serde_json::to_string(&proposed.weapon_mounts).unwrap_or_default();
    if old_w_json != new_w_json {
        let mount_value = |mounts: &[WeaponMount]| -> i64 {
            mounts.iter().map(|m| {
                let mount_base = match m.mount_kind.as_str() {
                    "single_turret" => 200_000,
                    "double_turret" => 500_000,
                    "triple_turret" => 1_000_000,
                    "barbette" => 4_000_000,
                    "bay" => 12_000_000,
                    _ => 500_000,
                };
                let weap_cost: i64 = m.weapons.iter().map(|w| {
                    let wl = w.to_ascii_lowercase();
                    if wl.contains("particle") { 4_000_000 }
                    else if wl.contains("beam") { 1_000_000 }
                    else if wl.contains("missile") { 750_000 }
                    else if wl.contains("pulse") { 500_000 }
                    else if wl.contains("sand") { 250_000 }
                    else { 500_000 }
                }).sum();
                mount_base + weap_cost
            }).sum()
        };
        let old_v = mount_value(&current.weapon_mounts);
        let new_v = mount_value(&proposed.weapon_mounts);
        items.push(RefitLineItem {
            item: format!(
                "Ordnance & Turret Refit ({} → {} mount(s))",
                current.weapon_mounts.len(),
                proposed.weapon_mounts.len()
            ),
            cost_cr: (new_v - old_v / 2).max(150_000),
            requires_shipyard: true,
        });
        needs_shipyard = true;
        days += 5;
    }

    if (proposed.fuel_capacity_tons - current.fuel_capacity_tons).abs() > 0.1 {
        let delta = (proposed.fuel_capacity_tons - current.fuel_capacity_tons).abs();
        items.push(RefitLineItem {
            item: format!(
                "Fuel tankage rework ({:.1}t → {:.1}t)",
                current.fuel_capacity_tons, proposed.fuel_capacity_tons
            ),
            cost_cr: (delta * 50_000.0).round() as i64,
            requires_shipyard: true,
        });
        needs_shipyard = true;
        days += 3;
    }
    if (proposed.cargo_capacity_tons - current.cargo_capacity_tons).abs() > 0.1 {
        let delta = (proposed.cargo_capacity_tons - current.cargo_capacity_tons).abs();
        items.push(RefitLineItem {
            item: format!(
                "Cargo bay rework ({:.1}t → {:.1}t)",
                current.cargo_capacity_tons, proposed.cargo_capacity_tons
            ),
            cost_cr: (delta * 25_000.0).round() as i64,
            requires_shipyard: true,
        });
        needs_shipyard = true;
        days += 2;
    }

    let has_changes = !items.is_empty();
    let total_cost_cr: i64 = items.iter().map(|i| i.cost_cr).sum();

    let (allowed, block_reason) = if needs_shipyard && !matches!(starport, 'A' | 'B') {
        (
            false,
            Some(format!(
                "Major ship refits require a Class A or B Starport shipyard (current starport is Class {starport})."
            )),
        )
    } else if needs_repair_yard && !matches!(starport, 'A' | 'B' | 'C') {
        (
            false,
            Some(format!(
                "Hull and critical repairs require at least a Class C Starport (current starport is Class {starport})."
            )),
        )
    } else {
        (true, None)
    };

    RefitQuote {
        allowed,
        block_reason,
        has_changes,
        total_cost_cr,
        days_required: days,
        items,
    }
}

/// Standard Type-A Free Trader (200 tons, J-1, M-1) starter ship template.
pub fn sample_free_trader() -> ShipSpec {
    ShipSpec {
        tonnage: 200,
        tl: 12,
        hull_config: HullConfig::Streamlined,
        hull_points_max: 80,
        armor_type: ArmorType::Crystaliron,
        armor_rating: 2,
        armor_tonnage: 5.0,
        thrust: 1,
        m_drive_tonnage: 2.0,
        jump_rating: 1,
        j_drive_tonnage: 10.0,
        power_plant_output: 75,
        power_plant_tonnage: 5.0,
        bridge_tonnage: 10.0,
        bridge_type: "standard".to_string(),
        computer_bandwidth: 5,
        computer_bis: false,
        computer_fib: false,
        sensor_grade: SensorGrade::Civilian,
        sensor_tonnage: 1.0,
        extended_sensor_array: false,
        stealth_dm: 0,
        hull_reflec: false,
        radiation_shielding: false,
        heat_shielding: false,
        self_sealing: false,
        reinforced_hull: false,
        light_hull: false,
        fuel_capacity_tons: 22.0,
        fuel_scoops: true,
        fuel_processor_tons: 1.0,
        staterooms: 10,
        high_staterooms: 0,
        luxury_staterooms: 0,
        barracks_berths: 0,
        low_berths: 20,
        emergency_low_berths: 0,
        common_areas_tonnage: 12.0,
        armory_tons: 0.0,
        medical_bay_tons: 0.0,
        workshop_tons: 0.0,
        briefing_room: false,
        library: false,
        probe_drones: 0,
        docking_bay_tons: 0.0,
        cargo_scoops: false,
        cargo_capacity_tons: 82.0,
        misc_tonnage: 0.0,
        weapon_mounts: vec![],
        purchase_price_cr: 46_088_000,
    }
}
