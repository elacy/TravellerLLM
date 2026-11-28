use traveller_db::SystemShipRecord;
use uuid::Uuid;

fn sample_corsair(transponder_active: bool) -> SystemShipRecord {
    SystemShipRecord {
        id: Uuid::new_v4(),
        campaign_id: Uuid::new_v4(),
        system_name: "Regina".into(),
        name: "VCS Sable Fang".into(),
        ship_class: "400t Vargr Corsair Raider".into(),
        role: "corsair".into(),
        tonnage: 400,
        tl: 12,
        hull_max: 160,
        hull_current: 160,
        armor: 4,
        thrust: 4,
        jump_rating: 2,
        sensor_grade: "military".into(),
        stealth_dm: -2,
        transponder_active,
        disposition: "hostile".into(),
        orbit_location: "Outer System / Sensor Shadow".into(),
        captain_name: "Corsair Krrsh-Varr".into(),
        faction: "Vargr Corsair Band".into(),
        cargo_hint: "Illicit salvage & hidden contraband holds".into(),
        comms_log: vec![],
        status: "in_system".into(),
        detected: transponder_active,
        detection_level: if transponder_active {
            "transponder".into()
        } else {
            "undetected".into()
        },
        captain_known: false,
        hostility_observed: false,
        observed_status: if transponder_active {
            "transponder_active".into()
        } else {
            "running_dark".into()
        },
    }
}

#[test]
fn system_ship_fog_of_war_redacts_dark_ships_captains_and_hidden_hostility() {
    // 1. Transponder-dark ship starts UNDETECTED -> completely hidden from player view
    let mut dark = sample_corsair(false);
    assert!(dark.to_player_view().is_none());

    // 2. Spotted at Minimal sensor detection -> track ID only, no name/class/captain/hostility
    dark.detected = true;
    dark.detection_level = "minimal".into();
    let v_min = dark.to_player_view().expect("detected contact must be visible");
    assert!(v_min.display_name.starts_with("Unidentified Contact ["));
    assert!(!v_min.display_name.contains("Sable Fang"));
    assert!(v_min.ship_class.is_none());
    assert!(v_min.tonnage.is_none());
    assert!(v_min.captain_name.is_none());
    assert!(v_min.faction.is_none());
    assert!(!v_min.hostility_observed);
    assert_ne!(v_min.observed_status, "hostile");

    // 3. Limited sensor scan -> reveals hull profile & drives, still hides armor/captain/hostility
    dark.detection_level = "limited".into();
    let v_lim = dark.to_player_view().unwrap();
    assert_eq!(v_lim.tonnage, Some(400));
    assert_eq!(v_lim.thrust, Some(4));
    assert!(v_lim.armor.is_none());
    assert!(v_lim.hull_max.is_none());
    assert!(v_lim.captain_name.is_none());
    assert_ne!(v_lim.observed_status, "hostile");

    // 4. Full sensor scan -> reveals hull/armor/stealth/notes, STILL hides captain & unproved hostility
    dark.detection_level = "full".into();
    let v_full = dark.to_player_view().unwrap();
    assert_eq!(v_full.hull_max, Some(160));
    assert_eq!(v_full.armor, Some(4));
    assert_eq!(v_full.stealth_dm, Some(-2));
    assert!(v_full.captain_name.is_none());
    assert_ne!(v_full.observed_status, "hostile");

    // 5. Captain only visible once broadcast/identified; Hostile only visible once observed
    dark.captain_known = true;
    dark.hostility_observed = true;
    let v_engaged = dark.to_player_view().unwrap();
    assert_eq!(v_engaged.captain_name.as_deref(), Some("Corsair Krrsh-Varr"));
    assert!(v_engaged.hostility_observed);
    assert_eq!(v_engaged.observed_status, "hostile");

    // 6. Transponder-active ship with secret 'hostile' disposition does NOT show 'hostile' or captain!
    let xpdr_hostile = sample_corsair(true);
    let v_xpdr = xpdr_hostile.to_player_view().unwrap();
    assert_eq!(v_xpdr.display_name, "VCS Sable Fang");
    assert!(v_xpdr.captain_name.is_none());
    assert!(!v_xpdr.hostility_observed);
    assert_eq!(v_xpdr.observed_status, "transponder_active");
}
