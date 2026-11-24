use rand::rngs::StdRng;
use rand::SeedableRng;
use traveller_rules::dice::RollMode;
use traveller_rules::sensors::{
    resolve_sensor_detection, DetectionLevel, SensorContext, SensorDetectionInput,
};
use traveller_rules::ship::SensorGrade;
use traveller_rules::universe::{hex_distance, parse_uwp};

/// Exercises High Guard 2024 sensor detection (dark stealthed ship vs loud merchant, plus
/// stealthed combat re-acquisition) and UWP trade-code + subsector route navigation.
#[test]
fn sensor_detection_stealth_scenarios_and_charted_space_navigation() {
    let mut rng = StdRng::seed_from_u64(99);

    // 1. Dark stealth corsair (passive, Thrust 0, stealth coating -6, transponder off)
    let dark_target = SensorDetectionInput {
        context: SensorContext::SpaceInitialDetection,
        operator_edu_dm: 1,
        operator_sensors_skill: 1,
        augment_or_software_dm: 0,
        observer_sensor_grade: SensorGrade::Civilian, // -2
        observer_tl: 12,
        observer_extended_array: false,
        observer_active_sensors: false, // passive sweep
        target_tl: 14,                  // observer has no TL advantage
        target_active_sensors: false,
        target_thrust_applied: 0,
        target_power_plant_active: true, // +1
        target_transponder_or_comms: false,
        target_stealth_armor_dm: -6, // -6 TL15 stealth coating
        target_fired_weapons: false,
        target_crit_damage_severity: 0,
        situational_dm: 0,
        roll_mode: RollMode::Normal,
    };
    let dark_res = resolve_sensor_detection(&mut rng, &dark_target);
    // 1 + 1 - 2 + 1 - 6 = -5
    assert_eq!(dark_res.roll.total_dm, -5);
    assert!(!dark_res.observer_signature_exposed);

    // 2. Loud merchant (transponder +6, active sensors +2, Thrust 2, TL12 spotted by TL15 patrol)
    let loud_target = SensorDetectionInput {
        context: SensorContext::SpaceInitialDetection,
        operator_edu_dm: 1,
        operator_sensors_skill: 2,
        augment_or_software_dm: 1,
        observer_sensor_grade: SensorGrade::Advanced, // +2
        observer_tl: 15,
        observer_extended_array: true, // +2
        observer_active_sensors: true,
        target_tl: 12,                  // +3 TL advantage
        target_active_sensors: true,    // +2
        target_thrust_applied: 2,       // +2
        target_power_plant_active: true,// +1
        target_transponder_or_comms: true, // +6
        target_stealth_armor_dm: 0,
        target_fired_weapons: false,
        target_crit_damage_severity: 0,
        situational_dm: 0,
        roll_mode: RollMode::Normal,
    };
    let loud_res = resolve_sensor_detection(&mut rng, &loud_target);
    // 1 + 2 + 1 + 2 + 3 + 2 + 2 + 2 + 1 + 6 = +22
    assert_eq!(loud_res.roll.total_dm, 22);
    assert!(loud_res.roll.success);
    assert_eq!(loud_res.detection_level, DetectionLevel::PinpointLock);
    assert!(loud_res.observer_signature_exposed);

    // 3. Stealthed ship re-acquisition after firing weapons (+2) and taking Severity 2 crit (+2 heat)
    let reacquire = SensorDetectionInput {
        context: SensorContext::SpaceStealthedReacquire,
        operator_edu_dm: 1,
        operator_sensors_skill: 2,
        augment_or_software_dm: 0,
        observer_sensor_grade: SensorGrade::Military, // 0
        observer_tl: 13,
        observer_extended_array: false,
        observer_active_sensors: true,
        target_tl: 12,                     // +1
        target_active_sensors: true,       // +2
        target_thrust_applied: 3,          // +3
        target_power_plant_active: true,   // not added in StealthedReacquire table
        target_transponder_or_comms: false,
        target_stealth_armor_dm: -4,
        target_fired_weapons: true,        // +2
        target_crit_damage_severity: 2,    // +2
        situational_dm: 0,
        roll_mode: RollMode::Normal,
    };
    let re_res = resolve_sensor_detection(&mut rng, &reacquire);
    // 1 + 2 + 0 + 1 + 2 + 3 - 4 + 2 + 2 = +9
    assert_eq!(re_res.roll.total_dm, 9);
    assert!(re_res.roll.success);

    // 4. Charted Space UWP parsing & canonical Core 2022 p. 261 Trade Codes across world archetypes
    let regina = parse_uwp("A788899-C").unwrap();
    assert_eq!(regina.starport, 'A');
    assert_eq!(regina.tech_level, 12);
    assert!(regina.trade_codes.contains(&"Ri".to_string()));
    assert!(regina.trade_codes.contains(&"Ht".to_string()));
    assert!(!regina.trade_codes.contains(&"Ga".to_string()));

    let industrial_core = parse_uwp("A875A87-E").unwrap();
    assert!(industrial_core.trade_codes.contains(&"Hi".to_string()));
    assert!(industrial_core.trade_codes.contains(&"In".to_string()));
    assert!(industrial_core.trade_codes.contains(&"Ht".to_string()));

    let belt = parse_uwp("C000543-A").unwrap();
    assert!(belt.trade_codes.contains(&"As".to_string()));
    assert!(belt.trade_codes.contains(&"Va".to_string()));
    assert!(belt.trade_codes.contains(&"Ni".to_string()));

    let garden = parse_uwp("A766654-B").unwrap();
    assert!(garden.trade_codes.contains(&"Ga".to_string()));
    assert!(garden.trade_codes.contains(&"Ag".to_string()));
    assert!(garden.trade_codes.contains(&"Ri".to_string()));

    let waterworld = parse_uwp("B56A744-9").unwrap();
    assert!(waterworld.trade_codes.contains(&"Wa".to_string()));

    // Multi-hop Jump-2 route check in Spinward Marches (Regina 1910 -> Roup 2007 -> Efate 1705)
    assert_eq!(hex_distance(19, 10, 19, 10), 0);
    assert_eq!(hex_distance(19, 10, 20, 10), 1);
    assert!(hex_distance(19, 10, 20, 7) <= 4);
    assert!(parse_uwp("INVALID").is_err());
}
