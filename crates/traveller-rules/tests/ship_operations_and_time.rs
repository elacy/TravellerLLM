use rand::rngs::StdRng;
use rand::SeedableRng;
use traveller_rules::calendar::{compute_advance, roll_jump_duration_hours, ImperialDate};
use traveller_rules::finance::compute_runway;
use traveller_rules::ship::{
    compute_running_costs, roll_poor_maintenance, roll_skipping_on_debts, sample_free_trader,
    validate_ship, HullConfig, Mortgage, WeaponMount,
};

/// Exercises ship design validation, mortgage/maintenance/life-support accounting, cash runway,
/// Imperial calendar jumps & maintenance period rollovers, poor maintenance crits, and debt skipping.
#[test]
fn ship_validation_finances_maintenance_and_time_lifecycle() {
    let mut rng = StdRng::seed_from_u64(2024);

    // 1. Baseline Type-A Free Trader validates cleanly under High Guard / Core 2022 rules
    let valid_ship = sample_free_trader();
    let baseline = validate_ship(&valid_ship);
    assert!(baseline.valid, "unexpected validation errors: {:?}", baseline.issues);
    assert_eq!(baseline.hardpoints_max, 2);
    assert_eq!(baseline.power_basic, 40);
    assert_eq!(baseline.power_m_drive, 20);
    assert_eq!(baseline.power_j_drive, 20);
    assert_eq!(baseline.min_engineers_required, 1);

    // 2. Mutated ship violating tonnage, hardpoints, jump fuel, power, and hull scoops
    let mut broken = sample_free_trader();
    broken.hull_config = HullConfig::Standard; // scoops on unstreamlined -> warning
    broken.cargo_capacity_tons = 120.0; // over tonnage -> error
    broken.jump_rating = 2; // requires 40t fuel, only has 22t -> error
    broken.power_plant_output = 30; // below basic systems (40) -> error
    broken.weapon_mounts = vec![
        WeaponMount {
            name: "Triple Laser".into(),
            mount_kind: "triple_turret".into(),
            weapons: vec!["Pulse Laser".into(); 3],
            tonnage: 1.0,
            power: 13,
            hardpoints: 1,
        },
        WeaponMount {
            name: "Missile Rack".into(),
            mount_kind: "triple_turret".into(),
            weapons: vec!["Missile Rack".into(); 3],
            tonnage: 1.0,
            power: 1,
            hardpoints: 1,
        },
        WeaponMount {
            name: "Illegal 3rd Turret".into(),
            mount_kind: "single_turret".into(),
            weapons: vec!["Sandcaster".into()],
            tonnage: 1.0,
            power: 1,
            hardpoints: 1,
        },
    ];
    let broken_report = validate_ship(&broken);
    assert!(!broken_report.valid);
    for cat in ["tonnage", "hardpoints", "power", "fuel"] {
        assert!(
            broken_report
                .issues
                .iter()
                .any(|i| i.category == cat && i.severity == "error"),
            "missing error for category '{cat}'"
        );
    }
    assert!(broken_report
        .issues
        .iter()
        .any(|i| i.category == "hull" && i.severity == "warning"));

    // Small craft (<100t) with a Jump Drive must be rejected
    let mut small_craft = sample_free_trader();
    small_craft.tonnage = 50;
    small_craft.jump_rating = 1;
    let sc_report = validate_ship(&small_craft);
    assert!(!sc_report.valid);
    assert!(sc_report.issues.iter().any(|i| i.category == "jump_drive"));

    // 3. Mortgages (Core p. 150): Ship Shares & Career Benefits (both term-reduction and remortgage)
    let m_shares = Mortgage::calculate(valid_ship.purchase_price_cr, 2, 0, false);
    assert_eq!(m_shares.monthly_payment_cr, 183_700); // (46,088,000 - 2,000,000) / 240
    assert_eq!(m_shares.periods_total, 480);

    let m_benefit_term = Mortgage::calculate(valid_ship.purchase_price_cr, 0, 1, false);
    assert_eq!(m_benefit_term.periods_total, 360); // 30 years left

    let m_benefit_remortgage = Mortgage::calculate(valid_ship.purchase_price_cr, 0, 1, true);
    assert_eq!(m_benefit_remortgage.periods_total, 480);
    assert!(m_benefit_remortgage.monthly_payment_cr < m_benefit_term.monthly_payment_cr);

    // 4. Combined 4-week running costs & runway forecast
    let costs = compute_running_costs(&valid_ship, Some(&m_shares), 4, 2, 15_000);
    assert_eq!(costs.mortgage_cr, 183_700);
    assert_eq!(costs.maintenance_cr, 3_841); // 0.1% of 46,088,000 / 12
    assert_eq!(costs.life_support_cr, 14_200); // 10*1000 + 4*1000 + 2*100
    assert_eq!(costs.crew_salaries_cr, 15_000);
    assert_eq!(costs.total_cr, 216_741);

    let runway = compute_runway(650_223, 25_000, costs.total_cr);
    assert!((runway.months_of_runway - 3.0).abs() < 1e-6);

    // 5. Imperial Calendar jump transit & 28-day Maintenance Period + year boundary rollover
    let mut date = ImperialDate::new(1105, 350, 9, 30);
    let mut total_maint_periods = 0;
    let mut total_years = 0;
    for _ in 0..4 {
        let jh = roll_jump_duration_hours(&mut rng);
        assert!((154..=184).contains(&jh)); // 148 + 6D6
        let adv = compute_advance(date, jh * 60);
        total_maint_periods += adv.maintenance_periods_crossed;
        total_years += adv.annual_boundaries_crossed;
        date = adv.to;
    }
    assert!(total_maint_periods >= 1);
    assert_eq!(total_years, 1);
    assert_eq!(date.year, 1106);

    // 6. Skipping maintenance eventually inflicts a Poor Maintenance critical hit
    let no_skip = roll_poor_maintenance(&mut rng, 0);
    assert!(!no_skip.suffered_crit);
    let mut saw_crit = false;
    for skipped in 1..=10 {
        let res = roll_poor_maintenance(&mut rng, skipped);
        if res.suffered_crit {
            assert!(res.crit_name.is_some());
            assert!(res.effect.is_some());
            saw_crit = true;
            break;
        }
    }
    assert!(saw_crit, "Expected a Poor Maintenance critical hit as skipped periods escalated");

    // 7. Skipping on Debts check (Core p. 153): heavily overdue high-value ship in high-law system
    let skip_check = roll_skipping_on_debts(
        &mut rng,
        1,          // -1 parsec
        0,          // no repaint/transponder alteration
        50_000_000, // +5 (per MCr10)
        true,       // +2 visited recently
        10,         // +4 (4-24 weeks overdue)
        9,          // +4 (Law 9 - 5)
    );
    // Total DM = -1 + 0 + 5 + 2 + 4 + 4 = +14 -> guaranteed 8+ (hunted)
    assert_eq!(skip_check.total_dm, 14);
    assert!(skip_check.hunted);
}
