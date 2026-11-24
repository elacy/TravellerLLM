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
    Planetoid,
    CloseStructure,
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
    pub armor_rating: i32,
    pub armor_tonnage: f64,
    pub thrust: u8,
    pub m_drive_tonnage: f64,
    pub jump_rating: u8,
    pub j_drive_tonnage: f64,
    pub power_plant_output: i32,
    pub power_plant_tonnage: f64,
    pub bridge_tonnage: f64,
    pub computer_bandwidth: u32,
    pub sensor_grade: SensorGrade,
    pub sensor_tonnage: f64,
    #[serde(default)]
    pub extended_sensor_array: bool,
    /// Stealth coating DM: 0, -2, -4, or -6 (High Guard 2024 p. 76–77)
    #[serde(default)]
    pub stealth_dm: i32,
    pub fuel_capacity_tons: f64,
    #[serde(default)]
    pub fuel_scoops: bool,
    #[serde(default)]
    pub fuel_processor_tons: f64,
    pub staterooms: u32,
    pub low_berths: u32,
    pub common_areas_tonnage: f64,
    pub cargo_capacity_tons: f64,
    #[serde(default)]
    pub misc_tonnage: f64,
    #[serde(default)]
    pub weapon_mounts: Vec<WeaponMount>,
    pub purchase_price_cr: i64,
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

    // 1. Tonnage balance
    let weapons_tonnage: f64 = spec.weapon_mounts.iter().map(|w| w.tonnage).sum();
    let stateroom_tonnage = f64::from(spec.staterooms) * 4.0 + f64::from(spec.low_berths) * 0.5;
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
            HullConfig::Standard | HullConfig::Distributed | HullConfig::Planetoid
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

/// Standard Type-A Free Trader (200 tons, J-1, M-1) starter ship template.
pub fn sample_free_trader() -> ShipSpec {
    ShipSpec {
        tonnage: 200,
        tl: 12,
        hull_config: HullConfig::Streamlined,
        hull_points_max: 80,
        armor_rating: 2,
        armor_tonnage: 5.0,
        thrust: 1,
        m_drive_tonnage: 2.0,
        jump_rating: 1,
        j_drive_tonnage: 10.0,
        power_plant_output: 75,
        power_plant_tonnage: 5.0,
        bridge_tonnage: 10.0,
        computer_bandwidth: 5,
        sensor_grade: SensorGrade::Civilian,
        sensor_tonnage: 1.0,
        extended_sensor_array: false,
        stealth_dm: 0,
        fuel_capacity_tons: 22.0,
        fuel_scoops: true,
        fuel_processor_tons: 1.0,
        staterooms: 10,
        low_berths: 20,
        common_areas_tonnage: 12.0,
        cargo_capacity_tons: 82.0,
        misc_tonnage: 0.0,
        weapon_mounts: vec![],
        purchase_price_cr: 46_088_000,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_trader_validates_cleanly_and_computes_mortgage_and_maintenance() {
        let ship = sample_free_trader();
        let report = validate_ship(&ship);
        assert!(report.valid, "issues: {:?}", report.issues);
        assert!(report.free_tonnage >= 0.0);
        assert_eq!(report.hardpoints_max, 2);
        assert_eq!(report.power_basic, 40);
        assert_eq!(report.power_m_drive, 20);
        assert_eq!(report.power_j_drive, 20);

        let mortgage = Mortgage::calculate(ship.purchase_price_cr, 2, 0, false);
        // (46,088,000 - 2,000,000) / 240 = 183,700
        assert_eq!(mortgage.monthly_payment_cr, 183_700);
        assert_eq!(mortgage.periods_total, 480);

        // Maintenance = 46,088,000 * 0.001 / 12 = 3,841 Cr
        assert_eq!(maintenance_period_cost(ship.purchase_price_cr), 3_841);

        // Life support: 10 staterooms * 1000 + 4 crew * 1000 + 2 low berths * 100 = 14,200
        assert_eq!(life_support_cost(10, 4, 2), 14_200);
    }

    #[test]
    fn over_tonnage_ship_fails_validation() {
        let mut ship = sample_free_trader();
        ship.cargo_capacity_tons = 120.0;
        let report = validate_ship(&ship);
        assert!(!report.valid);
        assert!(report.issues.iter().any(|i| i.category == "tonnage" && i.severity == "error"));
    }
}
