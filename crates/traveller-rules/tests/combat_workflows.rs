use std::collections::BTreeMap;

use rand::rngs::StdRng;
use rand::SeedableRng;
use traveller_rules::character::{Character, CharacterKind, Upp};
use traveller_rules::combat_personal::{
    next_personal_round, resolve_personal_attack, start_personal_combat, PersonalAttackInput,
    PersonalCombatant, PersonalRangeBand, PersonalWeapon, ReactionKind,
};
use traveller_rules::combat_space::{
    acquire_sensor_lock, advance_space_round, allocate_maneuver_thrust, ew_jam_missiles,
    launch_missile_salvo, resolve_space_attack, start_space_combat, SpaceCombatShip,
    SpaceRangeBand, SpaceWeaponAttackInput,
};
use uuid::Uuid;

#[test]
fn personal_combat_tracks_ammo_reactions_initiative_penalty_armor_ap_and_casualties() {
    let mut rng = StdRng::seed_from_u64(2022);
    let camp_id = Uuid::new_v4();
    let marine_id = Uuid::new_v4();
    let pirate_id = Uuid::new_v4();

    let mut skills = BTreeMap::new();
    skills.insert("Gun Combat (Slug)".into(), 3);
    let marine_char = Character {
        id: marine_id,
        campaign_id: camp_id,
        kind: CharacterKind::Player,
        name: "Sgt. Kael".into(),
        species: "Human".into(),
        age: 30,
        upp_max: Upp { str_val: 9, dex_val: 10, end_val: 9, int_val: 8, edu_val: 8, soc_val: 7, psi_val: None },
        upp_current: Upp { str_val: 9, dex_val: 10, end_val: 9, int_val: 8, edu_val: 8, soc_val: 7, psi_val: None },
        skills,
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

    let combatants = vec![
        PersonalCombatant {
            character_id: marine_id,
            name: "Sgt. Kael".into(),
            side: "party".into(),
            initiative: 0,
            next_round_init_penalty: 0,
            armor: 12,
            cover_dm: 0,
            aim_bonus: 2,
            range_band: PersonalRangeBand::Short,
            weapon: PersonalWeapon {
                name: "Gauss Rifle".into(),
                skill: "Gun Combat (Slug)".into(),
                range_m: 600,
                damage_dice: 4,
                damage_bonus: 0,
                ap: 5,
                auto_rating: 3,
                stun: false,
                magazine_current: 4,
                magazine_max: 40,
            },
            conscious: true,
            alive: true,
            str_cur: 9,
            dex_cur: 10,
            end_cur: 9,
        },
        PersonalCombatant {
            character_id: pirate_id,
            name: "Corsair Raider".into(),
            side: "hostile".into(),
            initiative: 0,
            next_round_init_penalty: 0,
            armor: 6, // Gauss Rifle AP 5 leaves only 1 effective armor!
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
            str_cur: 7,
            dex_cur: 7,
            end_cur: 7,
        },
    ];

    let mut state = start_personal_combat(&mut rng, combatants, 2, 0);
    assert_eq!(state.round, 1);

    // Burst fire (Auto 3) consumes 3 rounds of 4, while defender Dives for Cover (-2 init next round)
    let pirate_init_before = state
        .combatants
        .iter()
        .find(|c| c.character_id == pirate_id)
        .unwrap()
        .initiative;

    let res = resolve_personal_attack(
        &mut rng,
        &mut state,
        Some(&marine_char),
        &PersonalAttackInput {
            attacker_id: marine_id,
            defender_id: pirate_id,
            burst_mode: true,
            reaction: ReactionKind::DiveForCover,
            defender_reaction_dm: 0,
            extra_dm: 4, // guarantee clean hit to verify armor-AP and END->DEX->STR cascade
        },
    )
    .unwrap();

    assert!(res.hit);
    assert_eq!(res.ammo_remaining, 1);
    assert_eq!(res.armor_applied, 1, "6 armor - AP 5 must yield 1 effective armor");
    assert!(res.net_damage > 0);
    assert!(res.end_lost > 0);

    // Next round applies the Dive for Cover -2 initiative penalty
    next_personal_round(&mut state);
    assert_eq!(state.round, 2);
    let pirate_init_after = state
        .combatants
        .iter()
        .find(|c| c.character_id == pirate_id)
        .unwrap()
        .initiative;
    assert_eq!(pirate_init_after, pirate_init_before - 2);

    // Firing another burst (3 rounds) when only 1 round remains MUST fail with reload error
    let err = resolve_personal_attack(
        &mut rng,
        &mut state,
        Some(&marine_char),
        &PersonalAttackInput {
            attacker_id: marine_id,
            defender_id: pirate_id,
            burst_mode: true,
            reaction: ReactionKind::None,
            defender_reaction_dm: 0,
            extra_dm: 0,
        },
    );
    assert!(err.is_err());
}

#[test]
fn high_guard_space_combat_thrust_pool_sensor_lock_missiles_ew_and_critical_hits() {
    let mut rng = StdRng::seed_from_u64(2024);
    let trader_id = Uuid::new_v4();
    let corsair_id = Uuid::new_v4();

    let ships = vec![
        SpaceCombatShip {
            ship_id: trader_id,
            name: "ISS Beowulf".into(),
            side: "party".into(),
            tonnage: 200,
            tl: 12,
            hull_current: 80,
            hull_max: 80,
            armor: 4,
            thrust_max: 2,
            thrust_remaining: 2,
            evasive_dm: 0,
            aid_gunners_dm: 0,
            power_available: 60,
            sensor_lock_targets: vec![],
            crits: BTreeMap::new(),
            initiative: 0,
            last_ten_pct_threshold: 0,
        },
        SpaceCombatShip {
            ship_id: corsair_id,
            name: "Vargr Corsair".into(),
            side: "hostile".into(),
            tonnage: 400,
            tl: 12,
            hull_current: 160,
            hull_max: 160,
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
        },
    ];

    let mut state = start_space_combat(&mut rng, SpaceRangeBand::Medium, ships, 1, 0);
    assert_eq!(state.round, 1);
    assert_eq!(state.range_band, SpaceRangeBand::Medium);

    // 1. Maneuver Step: Beowulf allocates 1 Thrust to Evade and 1 Thrust to Aid Gunners
    let man = allocate_maneuver_thrust(
        &mut rng,
        &mut state,
        trader_id,
        0, // range
        1, // evade
        2, // Pilot-2
        true,
    )
    .unwrap();
    assert!(man.contains("allocated Thrust"));
    let trader = state.ships.iter().find(|s| s.ship_id == trader_id).unwrap();
    assert_eq!(trader.thrust_remaining, 0);
    assert_eq!(trader.evasive_dm, 3); // 1 thrust + Pilot-2

    // 2. Sensor Lock (+2 DM against Corsair)
    let lock_roll = acquire_sensor_lock(
        &mut rng,
        &mut state,
        trader_id,
        corsair_id,
        4, // High Sensors skill so lock succeeds
        2, // EDU DM
    )
    .unwrap();
    assert!(lock_roll.success);
    assert!(state
        .ships
        .iter()
        .find(|s| s.ship_id == trader_id)
        .unwrap()
        .sensor_lock_targets
        .contains(&corsair_id));

    // 3. Fire triple-linked Pulse Laser turret with high Gunner skill + Sensor Lock -> triggers Effect 6+ Critical Hit!
    let atk = resolve_space_attack(
        &mut rng,
        &mut state,
        &SpaceWeaponAttackInput {
            attacker_ship_id: trader_id,
            target_ship_id: corsair_id,
            weapon_name: "Triple Pulse Laser".into(),
            linked_count: 3, // +2 DM per die
            damage_dice: 4,  // high damage to also cross 10% hull threshold
            is_laser: true,
            gunner_skill: 4,
            gunner_dex_dm: 2,
            fire_control_dm: 4,
            sandcaster_count_defending: 1,
            defender_gunner_skill: 1,
            called_shot: None,
        },
    )
    .unwrap();
    assert!(atk.hit);
    assert!(atk.net_hull_damage > 0);
    assert!(
        !atk.crits_inflicted.is_empty(),
        "Effect 6+ or 10% hull loss must inflict High Guard Critical Hits"
    );

    // 4. Launch a 6-missile salvo from Medium range (2 rounds flight) and EW jam it
    let salvo = launch_missile_salvo(&mut state, corsair_id, trader_id, 6, 12);
    assert_eq!(salvo.rounds_to_impact, 2);
    let salvo_id = salvo.id;

    let (ew_roll, destroyed) = ew_jam_missiles(&mut rng, &mut state, salvo_id, 3, 2).unwrap();
    if ew_roll.success {
        assert!(destroyed >= 1);
    }

    // 5. Advance 2 rounds -> salvo reaches impact and resolves automatically, thrust pools reset
    let log1 = advance_space_round(&mut rng, &mut state);
    assert_eq!(state.round, 2);
    assert!(log1.iter().any(|l| l.contains("Round 2")));
    let _log2 = advance_space_round(&mut rng, &mut state);
    assert_eq!(state.round, 3);
    assert!(
        state.active_missiles.is_empty(),
        "Medium-range missile salvo must have impacted or been jammed by Round 3"
    );
}
