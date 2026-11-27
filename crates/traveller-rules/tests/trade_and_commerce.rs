use rand::rngs::StdRng;
use rand::SeedableRng;
use traveller_rules::commerce::{
    check_item_availability, AvailabilityInput, LawResponse, LegalityCategory,
};
use traveller_rules::trade::{
    all_trade_goods, generate_world_market, modified_price_percent, parsec_rates,
    roll_speculative_sale,
};
use traveller_rules::universe::{parse_uwp, plan_jump_route, RouteNode};
use uuid::Uuid;

#[test]
fn end_to_end_speculative_trade_freight_passengers_and_mail_workflow() {
    let mut rng = StdRng::seed_from_u64(1105);

    // Core 2022 parsec rates & modified price table boundaries
    assert_eq!(parsec_rates(1).high_passage, 9_000);
    assert_eq!(parsec_rates(1).freight_per_ton, 1_000);
    assert_eq!(parsec_rates(2).freight_per_ton, 1_600);
    assert_eq!(modified_price_percent(8), (100, 80));
    assert_eq!(modified_price_percent(25), (15, 400));
    assert_eq!(all_trade_goods().len(), 35);

    // Regina (A788899-C, Ri Ht) -> Efate (A646930-D, Hi In Ht)
    let regina = parse_uwp("A788899-C").unwrap();
    let efate = parse_uwp("A646930-D").unwrap();
    let legal_market = generate_world_market(
        &mut rng,
        "Regina",
        &regina,
        "Green",
        "Efate",
        &efate,
        2,
        2, // Broker-2
        1, // Steward-1
        false,
    );

    assert_eq!(legal_market.system_name, "Regina");
    assert!(
        legal_market.speculative_goods.iter().all(|g| !g.illegal),
        "Legal market must exclude illegal goods"
    );
    // Common goods (6) + Ht goods (Advanced Electronics, Cybernetics, etc.) must be present
    assert!(legal_market.speculative_goods.len() >= 8);
    assert_eq!(legal_market.passengers.rates.high_passage, 14_000);

    // Verify illegal market surfaces restricted goods on Ht world (Illegal Cybernetics / Weapons)
    let black_market = generate_world_market(
        &mut rng,
        "Regina",
        &regina,
        "Green",
        "Efate",
        &efate,
        2,
        2,
        1,
        true,
    );
    assert!(black_market.speculative_goods.iter().any(|g| g.illegal));

    // Sell an offered speculative good on Efate and verify modified price calculation
    let offer = &legal_market.speculative_goods[0];
    let (sale_pct, unit_sale_cr, roll_total) =
        roll_speculative_sale(&mut rng, offer.d66, &efate, "Green", 2, 2).unwrap();
    assert!((10..=400).contains(&sale_pct));
    assert!(unit_sale_cr > 0);
    assert!(roll_total >= -10);
}

#[test]
fn csc_2023_availability_black_market_and_law_enforcement_response() {
    let mut rng = StdRng::seed_from_u64(777);
    let regina = parse_uwp("A788899-C").unwrap(); // High-tech, Law 9, Starport A, Pop 8

    // 1. Common legal item at world TL on Regina with Broker-2 & 2x price succeeds easily
    let legal = check_item_availability(
        &mut rng,
        &regina,
        &AvailabilityInput {
            item_name: "Wafer Jack (TL12)".into(),
            item_tl: 12,
            base_cost_cr: 5_000,
            specialized: false,
            military_use: false,
            category: LegalityCategory::Category2Civilian,
            black_market: false,
            willing_price_multiplier: 2,
            previous_attempts_this_month: 0,
            skill_dm: 3,
            stat_dm: 2,
            referee_dm: 0,
        },
    );
    assert_eq!(legal.final_unit_price_cr, 10_000);
    assert_eq!(legal.law_response, LawResponse::None);

    // 2. Prohibited TL15 military item on Black Market on Law-9 world with unskilled buyer:
    // huge negative DMs guarantee failure with Effect <= -2 -> triggers Law Enforcement Response
    let illegal = check_item_availability(
        &mut rng,
        &regina,
        &AvailabilityInput {
            item_name: "FGMP-15".into(),
            item_tl: 15,
            base_cost_cr: 400_000,
            specialized: true,
            military_use: true,
            category: LegalityCategory::Prohibited,
            black_market: true,
            willing_price_multiplier: 1,
            previous_attempts_this_month: 2,
            skill_dm: -3,
            stat_dm: 0,
            referee_dm: 0,
        },
    );
    assert!(!illegal.available);
    assert_eq!(illegal.final_unit_price_cr, 400_000 * 20);
    assert_ne!(illegal.law_response, LawResponse::None);
    assert!(illegal.law_response_roll.is_some());

    // 3. Barren Pop-0 world never has commercial availability
    let barren = parse_uwp("X000000-0").unwrap();
    let res = check_item_availability(
        &mut rng,
        &barren,
        &AvailabilityInput {
            item_name: "Autopistol".into(),
            item_tl: 5,
            base_cost_cr: 200,
            specialized: false,
            military_use: false,
            category: LegalityCategory::Category2Civilian,
            black_market: false,
            willing_price_multiplier: 1,
            previous_attempts_this_month: 0,
            skill_dm: 2,
            stat_dm: 2,
            referee_dm: 0,
        },
    );
    assert!(!res.available);
}

#[test]
fn jump_route_planner_avoids_red_zones_and_unfueled_dead_ends() {
    let start_id = Uuid::new_v4();
    let red_mid_id = Uuid::new_v4();
    let no_fuel_id = Uuid::new_v4();
    let safe_mid_id = Uuid::new_v4();
    let goal_id = Uuid::new_v4();

    // Linear-ish 2-parsec gap with 3 middle candidates:
    // - red_mid: Red Zone (must be avoided when avoid_red_zones = true)
    // - no_fuel: Class E starport with 0 gas giants (must be avoided when require_refuel = true)
    // - safe_mid: Class C starport with 1 gas giant (valid refueling stop!)
    let nodes = vec![
        RouteNode {
            id: start_id,
            name: "Regina".into(),
            hex: "1910".into(),
            uwp: "A788899-C".into(),
            starport: 'A',
            gas_giants: 1,
            travel_zone: "Green".into(),
            global_x: 0,
            global_y: 0,
        },
        RouteNode {
            id: red_mid_id,
            name: "Interdicted World".into(),
            hex: "2010".into(),
            uwp: "B555555-A".into(),
            starport: 'B',
            gas_giants: 1,
            travel_zone: "Red".into(),
            global_x: 1,
            global_y: 0,
        },
        RouteNode {
            id: no_fuel_id,
            name: "Dry Rock".into(),
            hex: "2011".into(),
            uwp: "E100200-5".into(),
            starport: 'E',
            gas_giants: 0,
            travel_zone: "Green".into(),
            global_x: 1,
            global_y: 1,
        },
        RouteNode {
            id: safe_mid_id,
            name: "Safe Waypoint".into(),
            hex: "1911".into(),
            uwp: "C666666-9".into(),
            starport: 'C',
            gas_giants: 1,
            travel_zone: "Green".into(),
            global_x: 0,
            global_y: 1,
        },
        RouteNode {
            id: goal_id,
            name: "Target System".into(),
            hex: "2111".into(),
            uwp: "A646930-D".into(),
            starport: 'A',
            gas_giants: 1,
            travel_zone: "Green".into(),
            global_x: 2,
            global_y: 1,
        },
    ];

    // Jump-1 with Red Zone avoidance + Refuel required cannot reach (2,1) from (0,1) in 1 jump
    // unless Jump-2 is available, OR directly Jump-2 from (0,0) to (2,1) is 2 parsecs!
    let j1_blocked = plan_jump_route(&nodes, start_id, goal_id, 1, true, true);
    assert!(
        j1_blocked.is_none(),
        "Jump-1 should fail when all 1-parsec bridges are Red Zone or fuel-less"
    );

    // Jump-1 allowing Red Zones succeeds via Interdicted World
    let j1_via_red = plan_jump_route(&nodes, start_id, goal_id, 1, false, true).unwrap();
    assert_eq!(j1_via_red.total_jumps, 2);
    assert_eq!(j1_via_red.legs[0].to.name, "Interdicted World");

    // Jump-2 with strict safety + refuel jumps directly or via Safe Waypoint
    let j2_safe = plan_jump_route(&nodes, start_id, goal_id, 2, true, true).unwrap();
    assert_eq!(j2_safe.total_jumps, 1);
    assert_eq!(j2_safe.legs[0].to.name, "Target System");
    assert!(j2_safe.legs[0].refuel_options.iter().any(|s| s.contains("Refined")));
}
