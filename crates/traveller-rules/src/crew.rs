use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::chargen::{finalize_lifepath_character, muster_out, run_lifepath_term, start_lifepath, Career};
use crate::character::{Character, CharacterKind};
use crate::universe::Uwp;

/// Core 2022 p. 154 standard monthly (4-week Maintenance Period) crew salaries.
pub fn standard_crew_salary(role: &str) -> i64 {
    match role.trim().to_lowercase().as_str() {
        "pilot" => 6_000,
        "astrogator" => 5_000,
        "engineer" => 4_000,
        "medic" => 3_000,
        "steward" => 2_000,
        "gunner" => 1_000,
        "marine" | "security" => 1_000,
        "sensor_operator" | "sensors" => 3_000,
        _ => 2_000,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewCandidate {
    pub role: String,
    pub career: Career,
    pub terms: u16,
    pub asking_salary_cr: i64,
    pub character: Character,
}

const FIRST_NAMES: &[&str] = &[
    "Kael", "Sora", "Darius", "Talia", "Marek", "Vesper", "Joren", "Nadia",
    "Corvus", "Linnea", "Bram", "Yuki", "Soren", "Rhea", "Kira", "Orin",
];
const LAST_NAMES: &[&str] = &[
    "Vance", "Kowalski", "Sato", "Mercer", "Thorne", "Voss", "Al-Mansoor",
    "Reyes", "Kestrel", "Barrow", "Okafor", "Lindqvist", "Solis", "Drummond",
];

fn career_and_skill_for_role(role: &str) -> (Career, &'static str) {
    match role.trim().to_lowercase().as_str() {
        "pilot" => (Career::Navy, "Pilot (Spacecraft)"),
        "astrogator" => (Career::Scout, "Astrogation"),
        "engineer" => (Career::Merchant, "Engineer (J-Drive)"),
        "medic" => (Career::Scholar, "Medic"),
        "steward" => (Career::Merchant, "Steward"),
        "gunner" => (Career::Navy, "Gunner (Turret)"),
        "marine" | "security" => (Career::Marine, "Gun Combat (Slug)"),
        "sensor_operator" | "sensors" => (Career::Scout, "Electronics (sensors)"),
        _ => (Career::Citizen, "Mechanic"),
    }
}

/// Generate a realistic Starport Hiring Hall pool scaled by Starport Class & Population,
/// using the real MgT2e Lifepath generator so every hireling has authentic UPP, age, skills, and history.
pub fn generate_hiring_hall<R: Rng + ?Sized>(
    rng: &mut R,
    campaign_id: Uuid,
    world: &Uwp,
    roles: &[String],
) -> Vec<CrewCandidate> {
    let base_count = match world.starport {
        'A' => 6,
        'B' => 5,
        'C' => 3,
        'D' => 2,
        _ => 1,
    };
    let count = if roles.is_empty() {
        base_count
    } else {
        roles.len().min(8)
    };
    let default_roles = [
        "Pilot",
        "Astrogator",
        "Engineer",
        "Gunner",
        "Medic",
        "Steward",
        "Marine",
        "Sensors",
    ];

    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let role = roles
            .get(i)
            .cloned()
            .unwrap_or_else(|| default_roles[i % default_roles.len()].to_string());
        let (career, primary_skill) = career_and_skill_for_role(&role);
        let name = format!(
            "{} {}",
            FIRST_NAMES[rng.random_range(0..FIRST_NAMES.len())],
            LAST_NAMES[rng.random_range(0..LAST_NAMES.len())]
        );
        let terms = rng.random_range(1..=4);
        let mut session = start_lifepath(rng, name, "Human".into(), vec![primary_skill.into()]);
        for _ in 0..terms {
            run_lifepath_term(rng, &mut session, career, true);
        }
        // Guarantee competence in the advertised ship station
        let entry = session.skills.entry(primary_skill.to_string()).or_insert(1);
        if *entry < 1 {
            *entry = 1;
        }
        muster_out(rng, &mut session, 1);

        let base_salary = standard_crew_salary(&role);
        let skill_lvl = session.skills.get(primary_skill).copied().unwrap_or(1);
        let asking_salary_cr = base_salary + i64::from((skill_lvl - 1).max(0)) * 1_000;

        let character = finalize_lifepath_character(
            campaign_id,
            CharacterKind::Crew,
            &session,
            asking_salary_cr,
            Some(role.clone()),
        );
        out.push(CrewCandidate {
            role,
            career,
            terms,
            asking_salary_cr,
            character,
        });
    }
    out
}
