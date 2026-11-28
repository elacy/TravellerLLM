use rand::rngs::StdRng;
use rand::SeedableRng;
use traveller_rules::character::CharacterKind;
use traveller_rules::chargen::{
    all_background_skills, all_careers, all_species, finalize_lifepath_character, muster_out,
    run_lifepath_term, run_lifepath_term_detailed, run_pre_career, set_background_skills,
    start_lifepath, Career, PreCareerChoice, SkillTableChoice,
};
use traveller_rules::crew::{generate_hiring_hall, standard_crew_salary};
use traveller_rules::universe::parse_uwp;
use uuid::Uuid;

#[test]
fn full_lifepath_career_with_aging_pension_and_finalization() {
    let mut rng = StdRng::seed_from_u64(4242);
    let campaign_id = Uuid::new_v4();

    // Start a brand-new 18-year-old Traveller with custom background skills
    let mut session = start_lifepath(
        &mut rng,
        "Captain Elena Vance".into(),
        "Human".into(),
        vec!["Vacc Suit".into(), "Electronics (computers)".into(), "Admin".into()],
    );
    assert_eq!(session.age, 18);
    assert_eq!(session.terms_completed, 0);
    assert!(!session.skills.is_empty());

    // Run 6 terms across Navy and Merchant careers -> reaches age 42 (triggers Aging rolls at 34+)
    let careers = [
        Career::Navy,
        Career::Navy,
        Career::Navy,
        Career::Merchant,
        Career::Merchant,
        Career::Merchant,
    ];
    for (idx, career) in careers.into_iter().enumerate() {
        let entry = run_lifepath_term(&mut rng, &mut session, career, idx % 2 == 1);
        assert_eq!(entry.term, (idx + 1) as u16);
        assert_eq!(entry.age, 18 + ((idx + 1) as u16) * 4);
        if entry.age >= 34 {
            assert!(
                entry.aging_note.is_some(),
                "Core 2022 aging roll must occur at age 34+ (term {})",
                entry.term
            );
        } else {
            assert!(entry.aging_note.is_none());
        }
    }

    assert_eq!(session.terms_completed, 6);
    assert_eq!(session.age, 42);
    assert_eq!(session.log.len(), 6);

    // Muster out with 2 cash rolls; 6 terms earns Cr12,000/yr pension (10k + 2k)
    muster_out(&mut rng, &mut session, 2);
    assert!(session.mustered_out);
    assert_eq!(session.pension_cr, 12_000);

    // Calling muster_out twice is idempotent
    let cash_snapshot = session.cash_cr;
    muster_out(&mut rng, &mut session, 3);
    assert_eq!(session.cash_cr, cash_snapshot);

    // Finalize into a Postgres-ready Character record
    let character = finalize_lifepath_character(
        campaign_id,
        CharacterKind::Player,
        &session,
        0,
        Some("Captain".into()),
    );
    assert_eq!(character.name, "Captain Elena Vance");
    assert_eq!(character.age, 42);
    assert_eq!(character.kind, CharacterKind::Player);
    assert_eq!(character.upp_current, character.upp_max);
    assert!(character.skills.len() >= 3);
    assert!(character.notes.contains("6 terms"));
    assert!(character.notes.contains("Pension Cr12000/yr"));
}

#[test]
fn starport_hiring_hall_scales_with_starport_and_assigns_authentic_crew_salaries() {
    let mut rng = StdRng::seed_from_u64(9001);
    let campaign_id = Uuid::new_v4();

    // Class A starport Regina yields full hiring pool
    let regina = parse_uwp("A788899-C").unwrap();
    let regina_pool = generate_hiring_hall(&mut rng, campaign_id, &regina, &[]);
    assert_eq!(regina_pool.len(), 6);

    // Class E frontier starport yields a single candidate
    let frontier = parse_uwp("E893610-5").unwrap();
    let frontier_pool = generate_hiring_hall(&mut rng, campaign_id, &frontier, &[]);
    assert_eq!(frontier_pool.len(), 1);

    // Requesting specific stations guarantees competence and Core 2022 base+skill salaries
    let requested = vec!["Pilot".into(), "Engineer".into(), "Gunner".into(), "Medic".into()];
    let crew = generate_hiring_hall(&mut rng, campaign_id, &regina, &requested);
    assert_eq!(crew.len(), 4);

    assert_eq!(standard_crew_salary("Pilot"), 6_000);
    assert_eq!(standard_crew_salary("Astrogator"), 5_000);
    assert_eq!(standard_crew_salary("Engineer"), 4_000);
    assert_eq!(standard_crew_salary("Medic"), 3_000);
    assert_eq!(standard_crew_salary("Steward"), 2_000);
    assert_eq!(standard_crew_salary("Gunner"), 1_000);

    for cand in &crew {
        assert_eq!(cand.character.kind, CharacterKind::Crew);
        assert!(cand.character.age >= 22, "Lifepath crew must have at least 1 term");
        assert!(
            cand.asking_salary_cr >= standard_crew_salary(&cand.role),
            "Asking salary must be at least standard base salary for {}",
            cand.role
        );
        assert_eq!(cand.character.salary_cr, cand.asking_salary_cr);
    }
}

#[test]
fn species_catalog_background_skills_pre_career_and_detailed_terms() {
    let mut rng = StdRng::seed_from_u64(777);
    assert!(all_species().len() >= 10, "Must expose full Charted Space species list");
    assert!(all_background_skills().len() >= 15);
    assert_eq!(all_careers().len(), 12);

    // Zhodani rolls PSI; Aslan gets STR+2/DEX-2 and traits
    let zho = start_lifepath(&mut rng, "Zho".into(), "Human (Zhodani)".into(), vec![]);
    assert!(zho.upp.psi_val.is_some());
    let mut aslan = start_lifepath(&mut rng, "Htyowao".into(), "Aslan".into(), vec![]);
    assert!(aslan.species_traits.contains("dewclaw"));

    // Choosing custom background skills before Term 1 replaces defaults
    set_background_skills(&mut aslan, vec!["Pilot (Spacecraft)".into(), "Broker".into()]);
    assert!(aslan.background_skills_chosen);
    assert_eq!(aslan.skills.get("Broker"), Some(&0));

    // Pre-career education works before Term 1
    let note = run_pre_career(&mut rng, &mut aslan, PreCareerChoice::University);
    assert!(!note.is_empty());
    assert!(aslan.pre_career_done);

    // Detailed career term with Assignment & Personal Development table
    let entry = run_lifepath_term_detailed(
        &mut rng,
        &mut aslan,
        Career::Merchant,
        Some("Free Trader".into()),
        SkillTableChoice::Personal,
    );
    assert_eq!(entry.term, 1);
    assert!(!entry.skills_gained.is_empty());
}
