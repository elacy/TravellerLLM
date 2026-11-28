use std::collections::BTreeMap;

use rand::rngs::StdRng;
use rand::SeedableRng;
use traveller_rules::character::{
    Augment, Character, CharacterError, CharacterKind, Characteristic, ExpertSoftware, Upp,
};
use traveller_rules::dice::{resolve_task, Difficulty, ModifierItem, RollMode};
use uuid::Uuid;

fn make_traveller() -> Character {
    let mut skills = BTreeMap::new();
    skills.insert("Astrogation".to_string(), 1);
    skills.insert("Pilot (Small Craft)".to_string(), 2);

    Character {
        id: Uuid::new_v4(),
        campaign_id: Uuid::new_v4(),
        kind: CharacterKind::Player,
        name: "Elena Vance".to_string(),
        species: "Human".to_string(),
        age: 30,
        upp_max: Upp {
            str_val: 7,
            dex_val: 10,
            end_val: 8,
            int_val: 10,
            edu_val: 9,
            soc_val: 7,
            psi_val: None,
        },
        upp_current: Upp {
            str_val: 7,
            dex_val: 10,
            end_val: 8,
            int_val: 10,
            edu_val: 9,
            soc_val: 7,
            psi_val: None,
        },
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
    }
}

/// Exercises the full character task check + augment + wound + task-chain + XP progression loop
/// strictly through the public `traveller_rules` API.
#[test]
fn character_task_resolution_augments_wounds_and_xp_progression() {
    let mut rng = StdRng::seed_from_u64(1105);
    let mut traveller = make_traveller();

    // 1. Wounding DEX reduces current characteristic DM on physical rolls immediately
    traveller.upp_current.set(Characteristic::Dex, 4); // DM -1 instead of +1
    assert_eq!(traveller.upp_current.hex_code(), "748A97");
    assert_eq!(traveller.upp_max.hex_code(), "7A8A97");

    // Specialty fallback: having Pilot (Small Craft)-2 grants Pilot (Spacecraft)-0 (not -3)
    let pilot_mods = traveller.task_modifiers(
        Some("Pilot (Spacecraft)"),
        Characteristic::Dex,
        Difficulty::Average,
    );
    let pilot_roll = resolve_task(&mut rng, RollMode::Normal, Difficulty::Average, pilot_mods);
    assert_eq!(pilot_roll.total_dm, -1); // DEX 4 (-1) + Pilot (Spacecraft) 0

    // 2. Wafer Jack (Core p. 108 / CSC p. 61) supports INT/EDU Expert software ONLY
    traveller.upp_current.set(Characteristic::Dex, 10); // healed back to +1
    traveller.augments.push(Augment::WaferJack {
        tl: 13,
        computer_rating: 2,
        storage_bandwidth: 8,
        loaded_expert: vec![
            ExpertSoftware {
                skill: "Astrogation".into(),
                rating: 2,
                active: true,
            },
            ExpertSoftware {
                skill: "Gun Combat (Slug)".into(),
                rating: 2,
                active: true,
            },
        ],
    });

    // Mental check on possessed skill -> Astrogation (1) + EDU (+1) + Expert/2 assist (+1) = +3
    let astro_mods =
        traveller.task_modifiers(Some("Astrogation"), Characteristic::Edu, Difficulty::VeryDifficult);
    let astro_roll = resolve_task(&mut rng, RollMode::Boon, Difficulty::VeryDifficult, astro_mods);
    assert_eq!(astro_roll.total_dm, 3);
    assert_eq!(astro_roll.raw_dice.len(), 3);
    let dropped_low = *astro_roll.raw_dice.iter().min().unwrap();
    assert_eq!(
        astro_roll.natural_sum,
        astro_roll.raw_dice.iter().map(|&d| i32::from(d)).sum::<i32>() - i32::from(dropped_low)
    );

    // Physical check with Wafer Jack alone -> Expert does NOT apply; character remains unskilled (-3)
    let gun_wafer_mods = traveller.task_modifiers(
        Some("Gun Combat (Slug)"),
        Characteristic::Dex,
        Difficulty::Average,
    );
    let gun_wafer_roll = resolve_task(&mut rng, RollMode::Normal, Difficulty::Average, gun_wafer_mods);
    assert_eq!(gun_wafer_roll.total_dm, -2); // DEX (+1) + Unskilled (-3)

    // 3. Upgrading to Neural Jack (CSC p. 61) + Nerve-plug Response Rig + Peripheral Vision
    //    unlocks physical Expert software (Expert/2 -> skill 1) up to Very Difficult (12+),
    //    and chains the Astrogation Effect DM into the follow-up check.
    traveller.augments = vec![
        Augment::NeuralJack {
            tl: 13,
            computer_rating: 2,
            storage_bandwidth: 8,
            loaded_expert: vec![
                ExpertSoftware {
                    skill: "Gun Combat (Slug)".into(),
                    rating: 2,
                    active: true,
                },
                ExpertSoftware {
                    skill: "Electronics (sensors)".into(),
                    rating: 1, // Max difficulty: Difficult (10+)
                    active: true,
                },
            ],
        },
        Augment::NervePlugResponseRig { plugged_in: true }, // +2 DEX checks
        Augment::SkillAugmentation {
            skill: "Astrogation".into(),
        }, // +1 Astrogation (possessed at 0+)
    ];

    let mut gun_nj_mods = traveller.task_modifiers(
        Some("Gun Combat (Slug)"),
        Characteristic::Dex,
        Difficulty::VeryDifficult,
    );
    gun_nj_mods.push(ModifierItem {
        label: "Task Chain".into(),
        value: astro_roll.task_chain_dm,
    });
    let gun_nj_roll = resolve_task(&mut rng, RollMode::Bane, Difficulty::VeryDifficult, gun_nj_mods);
    // DEX (+1) + Neural Jack Expert/2 (1) + Nerve-plug (+2) + chain
    assert_eq!(gun_nj_roll.total_dm, 4 + astro_roll.task_chain_dm);
    let dropped_high = *gun_nj_roll.raw_dice.iter().max().unwrap();
    assert_eq!(
        gun_nj_roll.natural_sum,
        gun_nj_roll.raw_dice.iter().map(|&d| i32::from(d)).sum::<i32>() - i32::from(dropped_high)
    );

    // Expert/1 cannot assist beyond Difficult (10+) -> on VeryDifficult (12+) falls back to Unskilled (-3)
    let sensors_over_cap = traveller.task_modifiers(
        Some("Electronics (sensors)"),
        Characteristic::Edu,
        Difficulty::VeryDifficult,
    );
    assert_eq!(
        sensors_over_cap.iter().map(|m| m.value).sum::<i32>(),
        1 - 3
    );

    // 4. Per-skill activity XP & Companion 2024 exact doubling upgrade curve
    // Unlearned skill: 0 costs 1 XP, 1 costs 1 XP, 2 costs 2 XP, 3 costs 4 XP (total 8 XP)
    traveller.award_skill_xp("Broker", 7);
    assert_eq!(traveller.upgrade_skill("Broker", false).unwrap(), 0); // 6 left
    assert_eq!(traveller.upgrade_skill("Broker", false).unwrap(), 1); // 5 left
    assert_eq!(traveller.upgrade_skill("Broker", false).unwrap(), 2); // 3 left
    assert!(matches!(
        traveller.upgrade_skill("Broker", false),
        Err(CharacterError::InsufficientXp {
            needed: 4,
            available: 3
        })
    ));

    // Top up with 1 General XP to complete Broker-3
    traveller.general_xp = 1;
    assert_eq!(traveller.upgrade_skill("Broker", true).unwrap(), 3);
    assert_eq!(traveller.general_xp, 0);
    assert_eq!(*traveller.skill_xp.get("Broker").unwrap(), 0);

    // Characteristic upgrades: STR 7 -> 8 costs 8 XP; EDU 9 -> 10 costs 20 XP; SOC forbidden
    traveller.general_xp = 28;
    assert_eq!(
        traveller
            .upgrade_characteristic(Characteristic::Str, true)
            .unwrap(),
        8
    );
    assert_eq!(
        traveller
            .upgrade_characteristic(Characteristic::Edu, true)
            .unwrap(),
        10
    );
    assert_eq!(traveller.general_xp, 0);
    assert!(matches!(
        traveller.upgrade_characteristic(Characteristic::Soc, true),
        Err(CharacterError::SocNotTrainable)
    ));

    // 5. Core 2022 p. 62: Level 0 skills have NO specialisations; advancing 0 -> 1 requires one
    traveller.general_xp = 3;
    // Attempting to learn "Electronics (Computers)" when untrained stores unspecialised "Electronics-0"
    assert_eq!(traveller.upgrade_skill("Electronics (Computers)", true).unwrap(), 0);
    assert_eq!(traveller.skills.get("Electronics"), Some(&0));
    assert!(!traveller.skills.contains_key("Electronics (Computers)"));
    // Attempting to upgrade "Electronics-0" without choosing a specialisation fails
    assert!(matches!(
        traveller.upgrade_skill("Electronics", true),
        Err(CharacterError::SpecializationRequired(_))
    ));
    // Upgrading with a specialisation advances directly to Level 1 while preserving base Electronics-0
    assert_eq!(traveller.upgrade_skill("Electronics (Computers)", true).unwrap(), 1);
    assert_eq!(traveller.skills.get("Electronics (Computers)"), Some(&1));
    assert_eq!(traveller.skills.get("Electronics"), Some(&0));
    assert_eq!(traveller.raw_skill_level("Electronics (Sensors)"), Some(0));
}
