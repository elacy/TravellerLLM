use rand::rngs::StdRng;
use rand::SeedableRng;
use traveller_rules::traffic::generate_system_traffic;
use traveller_rules::universe::parse_uwp;

#[test]
fn system_traffic_scales_with_starport_bases_and_travel_zones() {
    let mut rng = StdRng::seed_from_u64(1105);

    // 1. Major Class-A capital Regina with Naval & Scout bases generates heavy traffic including Navy & Scout ships
    let regina = parse_uwp("A788899-C").unwrap();
    let regina_report = generate_system_traffic(
        &mut rng,
        "Regina",
        &regina,
        &["N".into(), "S".into()],
        "Green",
    );
    assert!(regina_report.ship_count >= 7);
    assert_eq!(regina_report.vessels.len(), regina_report.ship_count);
    assert!(regina_report.summary.contains("Regina"));
    assert!(regina_report
        .vessels
        .iter()
        .any(|v| v.role == "naval_patrol"));
    assert!(regina_report.vessels.iter().any(|v| v.role == "scout"));

    // 2. Frontier Class-E world has sparse traffic (0-2 ships)
    let frontier = parse_uwp("E893610-5").unwrap();
    let frontier_report = generate_system_traffic(&mut rng, "Forboldn", &frontier, &[], "Green");
    assert!(frontier_report.ship_count <= 3);

    // 3. Red Zone interdicted world includes hostile naval patrol enforcement,
    //    but its arrival summary NEVER leaks hidden hostility or dark-ship counts!
    let red_world = parse_uwp("C555555-9").unwrap();
    let red_report = generate_system_traffic(&mut rng, "Interdict", &red_world, &[], "Red");
    assert!((1..=3).contains(&red_report.ship_count));
    assert!(red_report
        .vessels
        .iter()
        .any(|v| v.disposition == "hostile"));
    assert!(
        !red_report.summary.to_lowercase().contains("hostile"),
        "Passive arrival summary must not leak hidden hostile disposition: {}",
        red_report.summary
    );
}
