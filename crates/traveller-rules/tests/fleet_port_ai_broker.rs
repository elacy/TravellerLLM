use std::collections::BTreeMap;

use rand::rngs::StdRng;
use rand::SeedableRng;
use serde_json::json;
use traveller_rules::character::{Augment, Character, CharacterKind, ExpertSoftware, Upp};
use traveller_rules::combat_personal::{
    execute_personal_ai_turn, start_personal_combat, PersonalCombatant, PersonalRangeBand,
    PersonalWeapon, ReactionKind,
};
use traveller_rules::combat_space::{
    execute_space_ai_turn, start_space_combat, SpaceCombatShip, SpaceRangeBand,
};
use traveller_rules::ship::{
    calculate_accrued_berthing_fee, calculate_refuel_quote, calculate_refit_cost,
    fleet_max_jump_rating, jump_fuel_for_distance, roll_berthing_fee, sample_free_trader,
    validate_ship, ArmorType, HullConfig, RefuelMode, SensorGrade,
};
use traveller_rules::trade::{compute_fleet_max_broker, search_local_broker};
use traveller_rules::universe::{build_spatial_memory_scope, parse_uwp};
use uuid::Uuid;

#[test]
fn fleet_jump_limit_refueling_berthing_and_spatial_memory_tags() {
    let mut rng = StdRng::seed_from_u64(1105);

    // 1. Fleet Jump Limit = lowest jump rating among all ships in the fleet
    let free_trader = sample_free_trader(); // J-1, 200t
    let mut far_trader = sample_free_trader();
    far_trader.jump_rating = 2;
    let mut scout = sample_free_trader();
    scout.tonnage = 100;
    scout.jump_rating = 3;

    assert_eq!(fleet_max_jump_rating(&[&far_trader, &scout]), 2);
    assert_eq!(fleet_max_jump_rating(&[&free_trader, &far_trader, &scout]), 1);
    assert!((jump_fuel_for_distance(200, 1) - 20.0).abs() < 0.01);
    assert!((jump_fuel_for_distance(200, 2) - 40.0).abs() < 0.01);

    // 2. Refueling rules across Starport classes & Gas Giant skimming
    let ref_a = calculate_refuel_quote('A', 1, &free_trader, 20.0, RefuelMode::Refined);
    assert!(ref_a.allowed && ref_a.total_cost_cr == 10_000);

    let ref_c = calculate_refuel_quote('C', 1, &free_trader, 20.0, RefuelMode::Refined);
    assert!(!ref_c.allowed, "Starport C cannot sell refined fuel");

    let unref_c = calculate_refuel_quote('C', 1, &free_trader, 20.0, RefuelMode::Unrefined);
    assert!(unref_c.allowed && unref_c.total_cost_cr == 2_000);

    let skim = calculate_refuel_quote('E', 1, &free_trader, 20.0, RefuelMode::GasGiantSkim);
    assert!(skim.allowed && skim.total_cost_cr == 0);

    let proc = calculate_refuel_quote('E', 0, &free_trader, 20.0, RefuelMode::ProcessOnboard);
    assert!(proc.allowed && proc.hours_required >= 24);

    // 3. Starport Berthing Fees (Core 2022 p. 235) & Incremental Stay Billing
    let fee_a = roll_berthing_fee(&mut rng, 'A', 2);
    assert!(fee_a.base_berthing_cr >= 1_000 && fee_a.base_berthing_cr <= 6_000);
    assert_eq!(fee_a.extra_days_cr, 1_000);
    assert_eq!(fee_a.total_cr, fee_a.base_berthing_cr + 1_000);
    let fee_e = roll_berthing_fee(&mut rng, 'E', 3);
    assert_eq!(fee_e.total_cr, 0);

    // Stateful stay duration billing: first 6 days prepaid by initial docking fee
    let t0 = 100_000_i64;
    let day6 = calculate_accrued_berthing_fee('A', t0, t0 + 6 * 1440, 6);
    assert_eq!(day6.newly_billable_days, 0);
    assert_eq!(day6.fee_cr, 0);

    let day7 = calculate_accrued_berthing_fee('A', t0, t0 + 7 * 1440, 6);
    assert_eq!(day7.newly_billable_days, 1);
    assert_eq!(day7.fee_cr, 500);

    // Subsequent days only bill the delta (no double-charging!)
    let day10 = calculate_accrued_berthing_fee('A', t0, t0 + 10 * 1440, 7);
    assert_eq!(day10.newly_billable_days, 3);
    assert_eq!(day10.fee_cr, 1_500);

    // Starport E charges Cr0 regardless of stay duration
    let day15_e = calculate_accrued_berthing_fee('E', t0, t0 + 15 * 1440, 6);
    assert_eq!(day15_e.newly_billable_days, 9);
    assert_eq!(day15_e.fee_cr, 0);

    // 4. Paid Shipyard Refit Cost & Starport Eligibility (incl. all Stateroom types, Hull Config & Stealth)
    let mut upgraded = free_trader.clone();
    upgraded.armor_type = ArmorType::BondedSuperdense;
    upgraded.armor_rating = 4;
    upgraded.sensor_grade = SensorGrade::Military;
    upgraded.stealth_dm = -4;
    upgraded.hull_reflec = true;
    upgraded.hull_config = HullConfig::CloseStructure;
    upgraded.high_staterooms = 1;
    upgraded.luxury_staterooms = 1;
    upgraded.barracks_berths = 2;
    upgraded.emergency_low_berths = 2;
    upgraded.cargo_capacity_tons = 60.0; // free space for new rooms
    let v = validate_ship(&upgraded);
    assert!(v.used_tonnage > validate_ship(&free_trader).used_tonnage - 22.0);
    let quote_a = calculate_refit_cost(&free_trader, 70, 0, &upgraded, 80, 0, 'A', 12);
    assert!(quote_a.allowed && quote_a.has_changes);
    assert!(quote_a.total_cost_cr >= 3_000_000, "Refit must charge real Cr costs");
    let quote_c = calculate_refit_cost(&free_trader, 70, 0, &upgraded, 80, 0, 'C', 12);
    assert!(!quote_c.allowed, "Major drive/armor refits must be blocked at Starport C");

    // 4b. Core 2022 Local Starport Broker Search (1D days + 5%/lvl commission)
    let broker_a = search_local_broker(&mut rng, 'A', 8, false);
    assert!((1..=6).contains(&broker_a.days_spent));
    if broker_a.found {
        assert!((1..=4).contains(&broker_a.broker_skill));
        assert_eq!(broker_a.commission_pct, (broker_a.broker_skill as u32) * 5);
    }
    let broker_x = search_local_broker(&mut rng, 'X', 0, false);
    assert!(!broker_x.found);

    // 5. Spatial & Polity Hindsight Memory Tags
    let regina = parse_uwp("A788899-C").unwrap();
    let scope = build_spatial_memory_scope("Regina", Some("Spinward Marches"), &regina, "Im");
    assert!(scope.local_tags.contains(&"system:regina".to_string()));
    assert!(scope.local_tags.contains(&"sector:spinward-marches".to_string()));
    assert!(scope.polity_tags.contains(&"polity:third-imperium".to_string()));
    assert!(scope.polity_tags.iter().any(|t| t.starts_with("gov:9-")));
    assert!(scope.polity_tags.contains(&"law:high".to_string()));
}

#[test]
fn fleet_max_broker_picks_highest_across_player_crew_neural_jacks_and_robots() {
    let camp_id = Uuid::new_v4();
    let mut pc_skills = BTreeMap::new();
    pc_skills.insert("Broker".into(), 1);
    let pc = Character {
        id: Uuid::new_v4(),
        campaign_id: camp_id,
        kind: CharacterKind::Player,
        name: "Capt. Vance".into(),
        species: "Human".into(),
        age: 30,
        upp_max: Upp::default(),
        upp_current: Upp::default(),
        skills: pc_skills,
        augments: vec![Augment::NeuralJack {
            tl: 13,
            computer_rating: 2,
            storage_bandwidth: 8,
            loaded_expert: vec![ExpertSoftware {
                skill: "Broker".into(),
                rating: 2,
                active: true,
            }],
        }],
        skill_xp: BTreeMap::new(),
        characteristic_xp: BTreeMap::new(),
        general_xp: 0,
        salary_cr: 0,
        loyalty: 0,
        assigned_ship_id: None,
        assigned_station: None,
        notes: String::new(),
    };

    // Unrelated high-skill story NPC MUST NOT count toward the player's fleet Broker!
    let mut npc_skills = BTreeMap::new();
    npc_skills.insert("Broker".into(), 5);
    let story_npc = Character {
        id: Uuid::new_v4(),
        campaign_id: camp_id,
        kind: CharacterKind::Npc,
        name: "Rival Megacorp Factor".into(),
        species: "Human".into(),
        age: 45,
        upp_max: Upp::default(),
        upp_current: Upp::default(),
        skills: npc_skills,
        augments: vec![],
        skill_xp: BTreeMap::new(),
        characteristic_xp: BTreeMap::new(),
        general_xp: 0,
        salary_cr: 0,
        loyalty: 0,
        assigned_ship_id: None,
        assigned_station: None,
        notes: String::new(),
    };

    // Without robot: PC has Broker-1 + Neural Jack Expert/2 (+1) = effective Broker-2
    let without_bot = compute_fleet_max_broker(&[(&pc, &[]), (&story_npc, &[])], &[]);
    assert_eq!(without_bot.max_broker_skill, 2);
    assert!(without_bot.broker_source.contains("Capt. Vance"));

    // With Executive Trade Droid (Broker-3) in character equipment -> jumps to Broker-3!
    let trade_droid = json!({
        "name": "Executive Megacorp Trade Droid (TL13)",
        "category": "robot_drone",
        "stats": { "broker": 3, "steward": 2 }
    });
    let with_bot = compute_fleet_max_broker(&[(&pc, &[trade_droid]), (&story_npc, &[])], &[]);
    assert_eq!(with_bot.max_broker_skill, 3);
    assert_eq!(with_bot.max_steward_skill, 2);
    assert!(with_bot.broker_source.contains("Trade Droid"));
}

#[test]
fn combat_enemy_ai_autonomously_fights_back_in_space_and_personal_combat() {
    let mut rng = StdRng::seed_from_u64(4242);

    // 1. Space Combat AI Turn: hostile Corsair maneuvers, locks sensors, and fires back
    let party_id = Uuid::new_v4();
    let corsair_id = Uuid::new_v4();
    let mut space_state = start_space_combat(
        &mut rng,
        SpaceRangeBand::Medium,
        vec![
            SpaceCombatShip {
                ship_id: party_id,
                name: "ISS Beowulf".into(),
                side: "party".into(),
                tonnage: 200,
                tl: 12,
                hull_max: 80,
                hull_current: 80,
                armor: 2,
                thrust_max: 2,
                thrust_remaining: 2,
                evasive_dm: 0,
                aid_gunners_dm: 0,
                power_available: 60,
                sensor_lock_targets: vec![],
                crits: BTreeMap::new(),
                initiative: 0,
                last_ten_pct_threshold: 0,
                maneuver_used: false,
                weapons_fired_this_round: 0,
                max_weapons_per_round: 1,
                sensor_action_used: false,
                missiles_launched_this_round: false,
            },
            SpaceCombatShip {
                ship_id: corsair_id,
                name: "Vargr Corsair".into(),
                side: "hostile".into(),
                tonnage: 400,
                tl: 12,
                hull_max: 160,
                hull_current: 160,
                armor: 4,
                thrust_max: 4,
                thrust_remaining: 4,
                evasive_dm: 0,
                aid_gunners_dm: 0,
                power_available: 120,
                sensor_lock_targets: vec![],
                crits: BTreeMap::new(),
                initiative: 0,
                last_ten_pct_threshold: 0,
                maneuver_used: false,
                weapons_fired_this_round: 0,
                max_weapons_per_round: 2,
                sensor_action_used: false,
                missiles_launched_this_round: false,
            },
        ],
        0,
        0,
    );
    let space_ai = execute_space_ai_turn(&mut rng, &mut space_state, 1, 1);
    assert_eq!(space_ai.actions.len(), 1);
    assert!(!space_ai.actions[0].actions.is_empty());
    assert_eq!(space_ai.actions[0].tactical_stance, "aggressive_assault");

    // 2. Personal Combat AI Turn: empty-magazine enemy reloads & takes cover, healthy enemy shoots back!
    let mut p_state = start_personal_combat(
        &mut rng,
        vec![
            PersonalCombatant {
                character_id: Uuid::new_v4(),
                name: "Capt. Vance".into(),
                side: "party".into(),
                initiative: 0,
                next_round_init_penalty: 0,
                armor: 6,
                cover_dm: 0,
                aim_bonus: 0,
                range_band: PersonalRangeBand::Short,
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
                str_cur: 8,
                dex_cur: 8,
                end_cur: 8,
                action_used_this_round: false,
            },
            PersonalCombatant {
                character_id: Uuid::new_v4(),
                name: "Corsair Gunner (Empty Mag)".into(),
                side: "hostile".into(),
                initiative: 0,
                next_round_init_penalty: 0,
                armor: 4,
                cover_dm: 0,
                aim_bonus: 0,
                range_band: PersonalRangeBand::Short,
                weapon: PersonalWeapon {
                    name: "Submachinegun".into(),
                    skill: "Gun Combat (Slug)".into(),
                    range_m: 30,
                    damage_dice: 3,
                    damage_bonus: 0,
                    ap: 0,
                    auto_rating: 3,
                    stun: false,
                    magazine_current: 0,
                    magazine_max: 30,
                },
                conscious: true,
                alive: true,
                str_cur: 7,
                dex_cur: 7,
                end_cur: 7,
                action_used_this_round: false,
            },
            PersonalCombatant {
                character_id: Uuid::new_v4(),
                name: "Corsair Marine".into(),
                side: "hostile".into(),
                initiative: 0,
                next_round_init_penalty: 0,
                armor: 4,
                cover_dm: 0,
                aim_bonus: 0,
                range_band: PersonalRangeBand::Short,
                weapon: PersonalWeapon {
                    name: "Assault Rifle".into(),
                    skill: "Gun Combat (Slug)".into(),
                    range_m: 200,
                    damage_dice: 3,
                    damage_bonus: 0,
                    ap: 0,
                    auto_rating: 2,
                    stun: false,
                    magazine_current: 20,
                    magazine_max: 20,
                },
                conscious: true,
                alive: true,
                str_cur: 8,
                dex_cur: 8,
                end_cur: 8,
                action_used_this_round: false,
            },
        ],
        0,
        0,
    );
    let p_ai = execute_personal_ai_turn(&mut rng, &mut p_state, ReactionKind::Dodge, 1);
    assert_eq!(p_ai.actions.len(), 2);
    assert!(p_ai.actions.iter().any(|a| a.decision == "reload_and_cover"));
    assert!(p_ai.actions.iter().any(|a| a.decision == "burst_fire"));
}
