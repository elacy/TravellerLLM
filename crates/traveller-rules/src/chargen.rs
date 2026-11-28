use std::collections::BTreeMap;

use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::character::{characteristic_dm, Character, CharacterKind, Characteristic, Upp};
use crate::dice::roll_nd6;

#[derive(Debug, Clone, Serialize)]
pub struct SpeciesSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub str_mod: i32,
    pub dex_mod: i32,
    pub end_mod: i32,
    pub int_mod: i32,
    pub edu_mod: i32,
    pub soc_mod: i32,
    pub psi_capable: bool,
    pub ignores_aging: bool,
    pub traits: &'static str,
}

/// Canonical Mongoose Traveller 2e playable species across Charted Space.
pub fn all_species() -> &'static [SpeciesSpec] {
    &[
        SpeciesSpec { id: "Human (Imperial)", name: "Human (Imperial)", str_mod: 0, dex_mod: 0, end_mod: 0, int_mod: 0, edu_mod: 0, soc_mod: 0, psi_capable: false, ignores_aging: false, traits: "Versatile Third Imperium Humaniti; balanced characteristics." },
        SpeciesSpec { id: "Human (Solomani)", name: "Human (Solomani)", str_mod: 0, dex_mod: 0, end_mod: 0, int_mod: 0, edu_mod: 0, soc_mod: 0, psi_capable: false, ignores_aging: false, traits: "Terran heritage; adaptable across frontier and core worlds." },
        SpeciesSpec { id: "Human (Vilani)", name: "Human (Vilani)", str_mod: 0, dex_mod: 0, end_mod: 0, int_mod: 0, edu_mod: 0, soc_mod: 0, psi_capable: false, ignores_aging: false, traits: "First Imperium lineage; strong bureaucratic & mercantile traditions." },
        SpeciesSpec { id: "Human (Zhodani)", name: "Human (Zhodani)", str_mod: 0, dex_mod: 0, end_mod: 0, int_mod: 0, edu_mod: 0, soc_mod: 0, psi_capable: true, ignores_aging: false, traits: "Zhodani Consulate citizen; tested for Psionics (PSI rolled at creation)." },
        SpeciesSpec { id: "Aslan", name: "Aslan", str_mod: 2, dex_mod: -2, end_mod: 0, int_mod: 0, edu_mod: 0, soc_mod: 0, psi_capable: false, ignores_aging: false, traits: "STR +2, DEX -2; retractable dewclaw (1D+2 melee), keen honor & territory instincts." },
        SpeciesSpec { id: "Vargr", name: "Vargr", str_mod: -1, dex_mod: 1, end_mod: -1, int_mod: 0, edu_mod: 0, soc_mod: 0, psi_capable: false, ignores_aging: false, traits: "STR -1, DEX +1, END -1; acute smell/hearing, bite attack, charisma-driven pack culture." },
        SpeciesSpec { id: "Droyne", name: "Droyne", str_mod: -2, dex_mod: 2, end_mod: -2, int_mod: 0, edu_mod: 0, soc_mod: 0, psi_capable: true, ignores_aging: false, traits: "STR -2, DEX +2, END -2; winged glider, ancient Coyns caste society, latent PSI." },
        SpeciesSpec { id: "Hiver", name: "Hiver", str_mod: -2, dex_mod: 1, end_mod: 0, int_mod: 2, edu_mod: 1, soc_mod: 0, psi_capable: false, ignores_aging: false, traits: "STR -2, DEX +1, INT +2, EDU +1; six-limbed manipulator, master strategist & negotiator." },
        SpeciesSpec { id: "K'kree", name: "K'kree", str_mod: 4, dex_mod: -2, end_mod: 2, int_mod: 0, edu_mod: 0, soc_mod: 0, psi_capable: false, ignores_aging: false, traits: "STR +4, DEX -2, END +2; large centauroid herbivore, herd-minded, claustrophobic." },
        SpeciesSpec { id: "Bwap", name: "Bwap", str_mod: -2, dex_mod: 1, end_mod: 0, int_mod: 1, edu_mod: 1, soc_mod: 0, psi_capable: false, ignores_aging: false, traits: "STR -2, DEX +1, INT +1, EDU +1; amphibious meticulous administrator (Admin-0)." },
        SpeciesSpec { id: "Virushi", name: "Virushi", str_mod: 4, dex_mod: -2, end_mod: 2, int_mod: 1, edu_mod: 0, soc_mod: 0, psi_capable: false, ignores_aging: false, traits: "STR +4, DEX -2, END +2, INT +1; massive gentle centauroid, pacifist outlook." },
        SpeciesSpec { id: "Uplifted Dolphin", name: "Uplifted Dolphin", str_mod: 2, dex_mod: 1, end_mod: 2, int_mod: 0, edu_mod: 0, soc_mod: 0, psi_capable: false, ignores_aging: false, traits: "STR +2, DEX +1, END +2; aquatic echolocation, grav-harness & waldo operator." },
        SpeciesSpec { id: "Uplifted Ape", name: "Uplifted Ape", str_mod: 2, dex_mod: 1, end_mod: 1, int_mod: 0, edu_mod: -1, soc_mod: 0, psi_capable: false, ignores_aging: false, traits: "STR +2, DEX +1, END +1, EDU -1; prehensile grip, natural climber & brawler." },
        SpeciesSpec { id: "Synthetic / Android (TL13+)", name: "Synthetic / Android (TL13+)", str_mod: 2, dex_mod: 0, end_mod: 2, int_mod: 0, edu_mod: 0, soc_mod: -2, psi_capable: false, ignores_aging: true, traits: "STR +2, END +2, SOC -2; immune to biological aging, direct computer interface." },
    ]
}

pub fn find_species(name: &str) -> Option<&'static SpeciesSpec> {
    let lower = name.trim().to_ascii_lowercase();
    all_species()
        .iter()
        .find(|s| s.id.to_ascii_lowercase() == lower || s.name.to_ascii_lowercase() == lower)
        .or_else(|| {
            if lower == "human" {
                all_species().first()
            } else {
                all_species()
                    .iter()
                    .find(|s| s.name.to_ascii_lowercase().contains(&lower))
            }
        })
}

/// Core 2022 p. 16 full Background Skills list (choose EDU DM + 3 at level 0).
pub fn all_background_skills() -> &'static [&'static str] {
    &[
        "Admin",
        "Animals (Handling)",
        "Art (Performer)",
        "Athletics (Dexterity)",
        "Athletics (Endurance)",
        "Carouse",
        "Drive (Wheel)",
        "Electronics (comms)",
        "Electronics (computers)",
        "Flyer (Grav)",
        "Language (Galanglic)",
        "Mechanic",
        "Medic",
        "Profession (Belter)",
        "Science (Physics)",
        "Seafarer (Personal)",
        "Streetwise",
        "Survival",
        "Vacc Suit",
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Career {
    Agent,
    Army,
    Citizen,
    Drifter,
    Entertainer,
    Marine,
    Merchant,
    Navy,
    Noble,
    Rogue,
    Scholar,
    Scout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillTableChoice {
    Service,
    Specialist,
    Personal,
    AdvancedEdu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreCareerChoice {
    None,
    University,
    MilitaryAcademy,
}

#[derive(Debug, Clone, Serialize)]
pub struct CareerSpec {
    pub career: Career,
    pub name: &'static str,
    pub assignments: [&'static str; 3],
    pub qual_stat: Characteristic,
    pub qual_target: i32,
    pub surv_stat: Characteristic,
    pub surv_target: i32,
    pub adv_stat: Characteristic,
    pub adv_target: i32,
    pub service_skills: [&'static str; 6],
    pub specialist_skills: [&'static str; 6],
    pub personal_dev: [&'static str; 6],
    pub advanced_edu_skills: [&'static str; 6],
    pub cash_table: [i64; 7],
    pub has_ship_benefit: bool,
}

pub fn all_careers() -> Vec<CareerSpec> {
    [
        Career::Agent,
        Career::Army,
        Career::Citizen,
        Career::Drifter,
        Career::Entertainer,
        Career::Marine,
        Career::Merchant,
        Career::Navy,
        Career::Noble,
        Career::Rogue,
        Career::Scholar,
        Career::Scout,
    ]
    .into_iter()
    .map(career_spec)
    .collect()
}

pub fn career_spec(career: Career) -> CareerSpec {
    match career {
        Career::Agent => CareerSpec {
            career,
            name: "Agent",
            assignments: ["Law Enforcement", "Intelligence", "Corporate"],
            qual_stat: Characteristic::Int,
            qual_target: 6,
            surv_stat: Characteristic::Int,
            surv_target: 6,
            adv_stat: Characteristic::Edu,
            adv_target: 6,
            service_skills: ["Streetwise", "Drive (Wheel)", "Investigate", "Flyer (Grav)", "Recon", "Gun Combat (Slug)"],
            specialist_skills: ["Investigate", "Recon", "Electronics (comms)", "Stealth", "Persuade", "Deception"],
            personal_dev: ["Gun Combat (Slug)", "DEX +1", "END +1", "Melee (Unarmed)", "INT +1", "Athletics (Dexterity)"],
            advanced_edu_skills: ["Advocate", "Electronics (comms)", "Electronics (computers)", "Medic", "Stealth", "Electronics (sensors)"],
            cash_table: [1_000, 2_000, 5_000, 7_500, 10_000, 25_000, 50_000],
            has_ship_benefit: false,
        },
        Career::Army => CareerSpec {
            career,
            name: "Army",
            assignments: ["Support", "Infantry", "Cavalry"],
            qual_stat: Characteristic::End,
            qual_target: 5,
            surv_stat: Characteristic::End,
            surv_target: 6,
            adv_stat: Characteristic::Edu,
            adv_target: 6,
            service_skills: ["Drive (Track)", "Athletics (Endurance)", "Gun Combat (Slug)", "Recon", "Melee (Blade)", "Heavy Weapons (Man Portable)"],
            specialist_skills: ["Tactics (Military)", "Leadership", "Gun Combat (Energy)", "Medic", "Stealth", "Survival"],
            personal_dev: ["STR +1", "DEX +1", "END +1", "Gambler", "Medic", "Melee (Unarmed)"],
            advanced_edu_skills: ["Tactics (Military)", "Electronics (comms)", "Navigation", "Explosives", "Engineer (Power)", "Medic"],
            cash_table: [2_000, 5_000, 10_000, 10_000, 10_000, 20_000, 30_000],
            has_ship_benefit: false,
        },
        Career::Citizen => CareerSpec {
            career,
            name: "Citizen",
            assignments: ["Corporate", "Worker", "Colonist"],
            qual_stat: Characteristic::Edu,
            qual_target: 5,
            surv_stat: Characteristic::Edu,
            surv_target: 5,
            adv_stat: Characteristic::Int,
            adv_target: 6,
            service_skills: ["Drive (Wheel)", "Flyer (Grav)", "Streetwise", "Melee (Unarmed)", "Steward", "Profession (Belter)"],
            specialist_skills: ["Admin", "Advocate", "Broker", "Electronics (computers)", "Mechanic", "Science (Economics)"],
            personal_dev: ["EDU +1", "INT +1", "Carouse", "Gambler", "Drive (Wheel)", "Jack-of-All-Trades"],
            advanced_edu_skills: ["Art (Visual Media)", "Advocate", "Diplomat", "Language (Galanglic)", "Electronics (computers)", "Medic"],
            cash_table: [1_000, 5_000, 10_000, 10_000, 10_000, 50_000, 100_000],
            has_ship_benefit: false,
        },
        Career::Drifter => CareerSpec {
            career,
            name: "Drifter",
            assignments: ["Barbarian", "Wanderer", "Scavenger"],
            qual_stat: Characteristic::End,
            qual_target: 2, // Automatic
            surv_stat: Characteristic::End,
            surv_target: 7,
            adv_stat: Characteristic::Int,
            adv_target: 7,
            service_skills: ["Athletics (Endurance)", "Melee (Unarmed)", "Recon", "Streetwise", "Stealth", "Survival"],
            specialist_skills: ["Vacc Suit", "Mechanic", "Jack-of-All-Trades", "Gun Combat (Slug)", "Navigation", "Carouse"],
            personal_dev: ["STR +1", "END +1", "DEX +1", "Language (Galanglic)", "Profession (Belter)", "Jack-of-All-Trades"],
            advanced_edu_skills: ["Mechanic", "Medic", "Navigation", "Vacc Suit", "Electronics (computers)", "Broker"],
            cash_table: [0, 0, 1_000, 2_000, 3_000, 4_000, 8_000],
            has_ship_benefit: false,
        },
        Career::Entertainer => CareerSpec {
            career,
            name: "Entertainer",
            assignments: ["Artist", "Journalist", "Performer"],
            qual_stat: Characteristic::Int,
            qual_target: 5,
            surv_stat: Characteristic::Int,
            surv_target: 5,
            adv_stat: Characteristic::Soc,
            adv_target: 6,
            service_skills: ["Art (Performer)", "Art (Visual Media)", "Carouse", "Deception", "Persuade", "Steward"],
            specialist_skills: ["Carouse", "Diplomat", "Gambler", "Investigate", "Language (Galanglic)", "Streetwise"],
            personal_dev: ["DEX +1", "INT +1", "SOC +1", "Language (Galanglic)", "Carouse", "Jack-of-All-Trades"],
            advanced_edu_skills: ["Advocate", "Art (Write)", "Deception", "Science (History)", "Streetwise", "Diplomat"],
            cash_table: [0, 0, 10_000, 10_000, 40_000, 40_000, 80_000],
            has_ship_benefit: false,
        },
        Career::Marine => CareerSpec {
            career,
            name: "Marine",
            assignments: ["Support", "Star Marine", "Ground Assault"],
            qual_stat: Characteristic::End,
            qual_target: 6,
            surv_stat: Characteristic::End,
            surv_target: 6,
            adv_stat: Characteristic::Edu,
            adv_target: 6,
            service_skills: ["Athletics (Dexterity)", "Vacc Suit", "Tactics (Military)", "Heavy Weapons (Artillery)", "Gun Combat (Slug)", "Stealth"],
            specialist_skills: ["Gun Combat (Energy)", "Melee (Blade)", "Leadership", "Medic", "Recon", "Explosives"],
            personal_dev: ["STR +1", "DEX +1", "END +1", "Gambler", "Melee (Unarmed)", "Melee (Blade)"],
            advanced_edu_skills: ["Medic", "Survival", "Explosives", "Engineer (Power)", "Pilot (Small Craft)", "Leadership"],
            cash_table: [2_000, 5_000, 5_000, 10_000, 20_000, 30_000, 40_000],
            has_ship_benefit: false,
        },
        Career::Merchant => CareerSpec {
            career,
            name: "Merchant",
            assignments: ["Merchant Marine", "Free Trader", "Broker"],
            qual_stat: Characteristic::Int,
            qual_target: 4,
            surv_stat: Characteristic::Int,
            surv_target: 5,
            adv_stat: Characteristic::Edu,
            adv_target: 6,
            service_skills: ["Drive (Wheel)", "Vacc Suit", "Broker", "Steward", "Electronics (comms)", "Persuade"],
            specialist_skills: ["Pilot (Spacecraft)", "Astrogation", "Engineer (J-Drive)", "Streetwise", "Admin", "Deception"],
            personal_dev: ["STR +1", "DEX +1", "END +1", "INT +1", "Language (Galanglic)", "Streetwise"],
            advanced_edu_skills: ["Engineer (M-Drive)", "Astrogation", "Electronics (computers)", "Pilot (Spacecraft)", "Admin", "Advocate"],
            cash_table: [1_000, 5_000, 10_000, 20_000, 20_000, 40_000, 40_000],
            has_ship_benefit: true,
        },
        Career::Navy => CareerSpec {
            career,
            name: "Navy",
            assignments: ["Line / Crew", "Engineering / Gunnery", "Flight"],
            qual_stat: Characteristic::Int,
            qual_target: 6,
            surv_stat: Characteristic::Int,
            surv_target: 5,
            adv_stat: Characteristic::Edu,
            adv_target: 6,
            service_skills: ["Pilot (Spacecraft)", "Vacc Suit", "Athletics (Dexterity)", "Gunner (Turret)", "Mechanic", "Gun Combat (Energy)"],
            specialist_skills: ["Astrogation", "Electronics (sensors)", "Engineer (Power)", "Engineer (M-Drive)", "Tactics (Naval)", "Leadership"],
            personal_dev: ["STR +1", "DEX +1", "END +1", "INT +1", "EDU +1", "SOC +1"],
            advanced_edu_skills: ["Admin", "Astrogation", "Electronics (sensors)", "Engineer (J-Drive)", "Medic", "Tactics (Naval)"],
            cash_table: [1_000, 5_000, 5_000, 10_000, 20_000, 50_000, 50_000],
            has_ship_benefit: false,
        },
        Career::Noble => CareerSpec {
            career,
            name: "Noble",
            assignments: ["Administrator", "Diplomat", "Dilettante"],
            qual_stat: Characteristic::Soc,
            qual_target: 10,
            surv_stat: Characteristic::Soc,
            surv_target: 5,
            adv_stat: Characteristic::Int,
            adv_target: 6,
            service_skills: ["Admin", "Advocate", "Electronics (computers)", "Diplomat", "Investigate", "Persuade"],
            specialist_skills: ["Carouse", "Deception", "Leadership", "Pilot (Small Craft)", "Broker", "Melee (Blade)"],
            personal_dev: ["STR +1", "DEX +1", "END +1", "Gambler", "Gun Combat (Slug)", "Melee (Blade)"],
            advanced_edu_skills: ["Admin", "Advocate", "Language (Vilani)", "Leadership", "Diplomat", "Electronics (computers)"],
            cash_table: [10_000, 10_000, 50_000, 50_000, 100_000, 100_000, 200_000],
            has_ship_benefit: true,
        },
        Career::Rogue => CareerSpec {
            career,
            name: "Rogue",
            assignments: ["Thief", "Enforcer", "Pirate"],
            qual_stat: Characteristic::Dex,
            qual_target: 6,
            surv_stat: Characteristic::Dex,
            surv_target: 6,
            adv_stat: Characteristic::Int,
            adv_target: 6,
            service_skills: ["Deception", "Recon", "Athletics (Dexterity)", "Gun Combat (Slug)", "Stealth", "Streetwise"],
            specialist_skills: ["Electronics (computers)", "Gambler", "Melee (Blade)", "Pilot (Small Craft)", "Astrogation", "Broker"],
            personal_dev: ["Carouse", "DEX +1", "END +1", "Gambler", "Melee (Blade)", "Gun Combat (Slug)"],
            advanced_edu_skills: ["Electronics (computers)", "Navigation", "Medic", "Investigate", "Broker", "Advocate"],
            cash_table: [0, 0, 10_000, 10_000, 50_000, 100_000, 100_000],
            has_ship_benefit: true,
        },
        Career::Scholar => CareerSpec {
            career,
            name: "Scholar",
            assignments: ["Field Researcher", "Scientist", "Physician"],
            qual_stat: Characteristic::Int,
            qual_target: 6,
            surv_stat: Characteristic::Edu,
            surv_target: 4,
            adv_stat: Characteristic::Int,
            adv_target: 6,
            service_skills: ["Drive (Wheel)", "Electronics (computers)", "Diplomat", "Medic", "Investigate", "Science (Physics)"],
            specialist_skills: ["Science (Xenology)", "Science (Robotics)", "Science (Cybernetics)", "Astrogation", "Electronics (sensors)", "Language (Vilani)"],
            personal_dev: ["INT +1", "EDU +1", "SOC +1", "DEX +1", "END +1", "Language (Vilani)"],
            advanced_edu_skills: ["Art (Write)", "Advocate", "Electronics (computers)", "Language (Galanglic)", "Engineer (Power)", "Medic"],
            cash_table: [5_000, 10_000, 20_000, 30_000, 40_000, 60_000, 100_000],
            has_ship_benefit: true,
        },
        Career::Scout => CareerSpec {
            career,
            name: "Scout",
            assignments: ["Courier", "Survey", "Explorer"],
            qual_stat: Characteristic::Int,
            qual_target: 5,
            surv_stat: Characteristic::End,
            surv_target: 7,
            adv_stat: Characteristic::Edu,
            adv_target: 7,
            service_skills: ["Pilot (Spacecraft)", "Survival", "Mechanic", "Astrogation", "Vacc Suit", "Gun Combat (Slug)"],
            specialist_skills: ["Electronics (sensors)", "Electronics (comms)", "Recon", "Jack-of-All-Trades", "Navigation", "Medic"],
            personal_dev: ["STR +1", "DEX +1", "END +1", "INT +1", "EDU +1", "Jack-of-All-Trades"],
            advanced_edu_skills: ["Medic", "Navigation", "Engineer (J-Drive)", "Electronics (computers)", "Pilot (Spacecraft)", "Jack-of-All-Trades"],
            cash_table: [20_000, 20_000, 30_000, 30_000, 50_000, 50_000, 50_000],
            has_ship_benefit: true,
        },
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifepathLogEntry {
    pub term: u16,
    pub age: u16,
    pub career: String,
    pub qualified: bool,
    pub survived: bool,
    pub advanced: bool,
    pub rank: u8,
    pub skills_gained: Vec<String>,
    pub event: String,
    pub aging_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifepathSession {
    pub id: Uuid,
    pub name: String,
    pub species: String,
    #[serde(default)]
    pub species_traits: String,
    pub age: u16,
    pub terms_completed: u16,
    pub upp: Upp,
    pub skills: BTreeMap<String, i32>,
    #[serde(default)]
    pub background_skill_slots: u32,
    #[serde(default)]
    pub background_skills_chosen: bool,
    #[serde(default)]
    pub pre_career_done: bool,
    #[serde(default)]
    pub pre_career_note: Option<String>,
    #[serde(default)]
    pub qualification_bonus_dm: i32,
    pub current_career: Option<Career>,
    #[serde(default)]
    pub current_assignment: Option<String>,
    pub current_rank: u8,
    pub benefit_rolls_earned: u32,
    pub cash_cr: i64,
    pub ship_shares: u32,
    pub career_ship_benefits: u32,
    pub pension_cr: i64,
    pub allies: Vec<String>,
    pub rivals: Vec<String>,
    pub mustered_out: bool,
    pub log: Vec<LifepathLogEntry>,
}

/// Start a new MgT2e lifepath character by rolling 6 characteristics (2D6 each), applying
/// authentic species modifiers/traits, and granting EDU DM + 3 background skills (Core 2022 p. 14–16).
pub fn start_lifepath<R: Rng + ?Sized>(
    rng: &mut R,
    name: String,
    species: String,
    background_skills: Vec<String>,
) -> LifepathSession {
    let mut r2 = || -> i32 {
        let d = roll_nd6(rng, 2);
        i32::from(d[0]) + i32::from(d[1])
    };
    let sp = find_species(&species);
    let canonical_species = sp.map(|s| s.name.to_string()).unwrap_or(species);
    let species_traits = sp.map(|s| s.traits.to_string()).unwrap_or_default();

    let clamp_stat = |v: i32| v.clamp(1, 15);
    let upp = Upp {
        str_val: clamp_stat(r2() + sp.map(|s| s.str_mod).unwrap_or(0)),
        dex_val: clamp_stat(r2() + sp.map(|s| s.dex_mod).unwrap_or(0)),
        end_val: clamp_stat(r2() + sp.map(|s| s.end_mod).unwrap_or(0)),
        int_val: clamp_stat(r2() + sp.map(|s| s.int_mod).unwrap_or(0)),
        edu_val: clamp_stat(r2() + sp.map(|s| s.edu_mod).unwrap_or(0)),
        soc_val: clamp_stat(r2() + sp.map(|s| s.soc_mod).unwrap_or(0)),
        psi_val: if sp.map(|s| s.psi_capable).unwrap_or(false) {
            Some(r2())
        } else {
            None
        },
    };

    let bg_limit = (characteristic_dm(upp.edu_val) + 3).max(1) as usize;
    let mut skills = BTreeMap::new();
    let background_skills_chosen = !background_skills.is_empty();
    let chosen: Vec<String> = if background_skills.is_empty() {
        all_background_skills()
            .iter()
            .take(bg_limit)
            .map(|s| (*s).to_string())
            .collect()
    } else {
        background_skills.into_iter().take(bg_limit).collect()
    };
    for sk in chosen {
        skills.entry(sk).or_insert(0);
    }
    if canonical_species == "Bwap" {
        skills.entry("Admin".to_string()).or_insert(0);
    }

    LifepathSession {
        id: Uuid::new_v4(),
        name,
        species: canonical_species,
        species_traits,
        age: 18,
        terms_completed: 0,
        upp,
        skills,
        background_skill_slots: bg_limit as u32,
        background_skills_chosen,
        pre_career_done: false,
        pre_career_note: None,
        qualification_bonus_dm: 0,
        current_career: None,
        current_assignment: None,
        current_rank: 0,
        benefit_rolls_earned: 0,
        cash_cr: 0,
        ship_shares: 0,
        career_ship_benefits: 0,
        pension_cr: 0,
        allies: vec![],
        rivals: vec![],
        mustered_out: false,
        log: vec![],
    }
}

/// Set the player's chosen Core 2022 background skills (allowed before Term 1 begins).
pub fn set_background_skills(session: &mut LifepathSession, chosen: Vec<String>) {
    if session.terms_completed > 0 || session.pre_career_done {
        return;
    }
    let limit = session.background_skill_slots.max(1) as usize;
    session.skills.clear();
    for sk in chosen.into_iter().take(limit) {
        session.skills.entry(sk).or_insert(0);
    }
    if session.species == "Bwap" {
        session.skills.entry("Admin".to_string()).or_insert(0);
    }
    session.background_skills_chosen = true;
}

/// Run Core 2022 Pre-Career Education (University or Military Academy) before Term 1.
pub fn run_pre_career<R: Rng + ?Sized>(
    rng: &mut R,
    session: &mut LifepathSession,
    choice: PreCareerChoice,
) -> String {
    if session.pre_career_done || session.terms_completed > 0 {
        return "Pre-career education is only available before Term 1.".into();
    }
    session.pre_career_done = true;
    match choice {
        PreCareerChoice::None => {
            let note = "Skipped pre-career education and entered the workforce at Age 18.".to_string();
            session.pre_career_note = Some(note.clone());
            note
        }
        PreCareerChoice::University => {
            let d = roll_nd6(rng, 2);
            let entry_roll = i32::from(d[0]) + i32::from(d[1]) + session.upp.dm(Characteristic::Edu);
            if entry_roll < 7 {
                let note = format!("University entrance failed (rolled {entry_roll} vs EDU 7+); entering careers at Age 18.");
                session.pre_career_note = Some(note.clone());
                return note;
            }
            session.age += 4;
            session.upp.edu_val = (session.upp.edu_val + 1).min(15);
            let uni_skills = ["Admin", "Advocate", "Broker", "Electronics (computers)", "Medic", "Science (Physics)"];
            let s1 = uni_skills[rng.random_range(0..uni_skills.len())];
            let s2 = uni_skills[(rng.random_range(0..uni_skills.len()) + 1) % uni_skills.len()];
            let g1 = gain_skill(&mut session.skills, s1);
            let g2 = gain_skill(&mut session.skills, s2);
            let gd = roll_nd6(rng, 2);
            let grad = i32::from(gd[0]) + i32::from(gd[1]) + session.upp.dm(Characteristic::Int);
            let note = if grad >= 11 {
                session.upp.edu_val = (session.upp.edu_val + 2).min(15);
                session.qualification_bonus_dm += 2;
                format!("University Graduated with Honours (roll {grad})! EDU +3 total, gained {g1} & {g2}, +2 DM to first career qualification.")
            } else if grad >= 7 {
                session.upp.edu_val = (session.upp.edu_val + 1).min(15);
                session.qualification_bonus_dm += 1;
                format!("University Graduated (roll {grad})! EDU +2 total, gained {g1} & {g2}, +1 DM to first career qualification.")
            } else {
                format!("Attended University (did not graduate, roll {grad}). EDU +1, gained {g1} & {g2}.")
            };
            session.pre_career_note = Some(note.clone());
            note
        }
        PreCareerChoice::MilitaryAcademy => {
            let d = roll_nd6(rng, 2);
            let entry_roll = i32::from(d[0]) + i32::from(d[1]) + session.upp.dm(Characteristic::End);
            if entry_roll < 8 {
                let note = format!("Military Academy admission failed (rolled {entry_roll} vs END 8+); entering careers at Age 18.");
                session.pre_career_note = Some(note.clone());
                return note;
            }
            session.age += 4;
            let acad_skills = ["Gun Combat (Slug)", "Leadership", "Tactics (Military)", "Vacc Suit", "Pilot (Spacecraft)", "Athletics (Endurance)"];
            let mut gained = Vec::new();
            for sk in acad_skills.iter().take(3) {
                session.skills.entry((*sk).to_string()).or_insert(0);
                gained.push(format!("{sk}-0"));
            }
            let gd = roll_nd6(rng, 2);
            let grad = i32::from(gd[0]) + i32::from(gd[1]) + session.upp.dm(Characteristic::Int);
            let note = if grad >= 11 {
                session.upp.soc_val = (session.upp.soc_val + 1).min(15);
                session.qualification_bonus_dm += 2;
                let lead = gain_skill(&mut session.skills, "Leadership");
                format!("Military Academy Graduated with Honours (roll {grad})! SOC +1, gained {}, {lead}, +2 DM to first career qualification.", gained.join(", "))
            } else if grad >= 8 {
                session.qualification_bonus_dm += 1;
                format!("Military Academy Graduated (roll {grad})! Gained {}, +1 DM to first career qualification.", gained.join(", "))
            } else {
                format!("Attended Military Academy (did not graduate, roll {grad}). Gained {}.", gained.join(", "))
            };
            session.pre_career_note = Some(note.clone());
            note
        }
    }
}

fn gain_skill_or_stat(upp: &mut Upp, skills: &mut BTreeMap<String, i32>, entry: &str) -> String {
    match entry {
        "STR +1" => {
            upp.str_val = (upp.str_val + 1).min(15);
            "STR +1".into()
        }
        "DEX +1" => {
            upp.dex_val = (upp.dex_val + 1).min(15);
            "DEX +1".into()
        }
        "END +1" => {
            upp.end_val = (upp.end_val + 1).min(15);
            "END +1".into()
        }
        "INT +1" => {
            upp.int_val = (upp.int_val + 1).min(15);
            "INT +1".into()
        }
        "EDU +1" => {
            upp.edu_val = (upp.edu_val + 1).min(15);
            "EDU +1".into()
        }
        "SOC +1" => {
            upp.soc_val = (upp.soc_val + 1).min(15);
            "SOC +1".into()
        }
        other => gain_skill(skills, other),
    }
}

fn gain_skill(skills: &mut BTreeMap<String, i32>, name: &str) -> String {
    let entry = skills.entry(name.to_string()).or_insert(-1);
    if *entry < 0 {
        *entry = 0;
    } else {
        *entry += 1;
    }
    format!("{name}-{}", *entry)
}

/// Run one 4-year MgT2e Career Term (Qualification if new career, Basic Training, Survival,
/// Mishap/Event, Advancement, Skill rolls, and Aging at 34+).
pub fn run_lifepath_term<R: Rng + ?Sized>(
    rng: &mut R,
    session: &mut LifepathSession,
    career: Career,
    prefer_specialist: bool,
) -> LifepathLogEntry {
    let table = if prefer_specialist {
        SkillTableChoice::Specialist
    } else {
        SkillTableChoice::Service
    };
    run_lifepath_term_detailed(rng, session, career, None, table)
}

/// Full MgT2e Career Term with Assignment and Skill Table selection (Service, Specialist,
/// Personal Development, or Advanced Education for EDU 8+).
pub fn run_lifepath_term_detailed<R: Rng + ?Sized>(
    rng: &mut R,
    session: &mut LifepathSession,
    career: Career,
    assignment: Option<String>,
    skill_table: SkillTableChoice,
) -> LifepathLogEntry {
    let spec = career_spec(career);
    let term_num = session.terms_completed + 1;
    let mut skills_gained = Vec::new();
    session.background_skills_chosen = true;
    session.pre_career_done = true;

    // 1. Qualification (only if entering a different career)
    let mut qualified = true;
    let entering_new = session.current_career != Some(career);
    let actual_career = if entering_new {
        let d = roll_nd6(rng, 2);
        let roll = i32::from(d[0]) + i32::from(d[1])
            + session.upp.dm(spec.qual_stat)
            + session.qualification_bonus_dm
            - (session.terms_completed as i32).min(3);
        session.qualification_bonus_dm = 0; // consumed on first qualification attempt
        if roll >= spec.qual_target {
            session.current_career = Some(career);
            session.current_assignment = assignment.or_else(|| Some(spec.assignments[0].to_string()));
            session.current_rank = 0;
            // Basic training: on first career, all 6 service skills at 0; later careers pick 1
            if session.terms_completed == 0 {
                for sk in spec.service_skills {
                    session.skills.entry(sk.to_string()).or_insert(0);
                    skills_gained.push(format!("{sk}-0 (Basic)"));
                }
            } else {
                let idx = rng.random_range(0..6);
                let sk = spec.service_skills[idx];
                session.skills.entry(sk.to_string()).or_insert(0);
                skills_gained.push(format!("{sk}-0 (Basic)"));
            }
            career
        } else {
            qualified = false;
            session.current_career = Some(Career::Drifter);
            session.current_assignment = Some(career_spec(Career::Drifter).assignments[1].to_string());
            session.current_rank = 0;
            Career::Drifter
        }
    } else {
        if let Some(a) = assignment {
            session.current_assignment = Some(a);
        }
        career
    };

    let active_spec = career_spec(actual_career);

    // 2. Term Skill Roll (Service / Specialist / Personal Dev / Advanced Education if EDU 8+)
    let effective_table = if skill_table == SkillTableChoice::AdvancedEdu && session.upp.edu_val < 8 {
        SkillTableChoice::Specialist
    } else {
        skill_table
    };
    let table: &[&str; 6] = match effective_table {
        SkillTableChoice::Service => &active_spec.service_skills,
        SkillTableChoice::Specialist => &active_spec.specialist_skills,
        SkillTableChoice::Personal => &active_spec.personal_dev,
        SkillTableChoice::AdvancedEdu => &active_spec.advanced_edu_skills,
    };
    let sk_idx = rng.random_range(0..6);
    skills_gained.push(gain_skill_or_stat(&mut session.upp, &mut session.skills, table[sk_idx]));

    // 3. Survival Roll
    let s_dice = roll_nd6(rng, 2);
    let s_total = i32::from(s_dice[0]) + i32::from(s_dice[1]) + session.upp.dm(active_spec.surv_stat);
    let survived = s_total >= active_spec.surv_target;

    let mut advanced = false;
    let event: String;

    if !survived {
        let mishap = roll_nd6(rng, 1)[0];
        event = match mishap {
            1 => {
                session.upp.end_val = (session.upp.end_val - 1).max(1);
                "Mishap: Severely injured in the line of duty (END -1); forced out of career.".into()
            }
            2 => {
                session.rivals.push(format!("Ex-{} Rival", active_spec.name));
                "Mishap: Betrayed by a colleague — gained a Rival and forced out.".into()
            }
            3 => "Mishap: Political fallout / scandal — discharged cleanly with 0 benefit rolls for this term.".into(),
            4 => {
                session.ship_shares += 1;
                "Mishap: Salvage dispute ended your career, but you walked away with +1 Ship Share.".into()
            }
            5 => {
                skills_gained.push(gain_skill(&mut session.skills, "Streetwise"));
                "Mishap: Whistleblower incident — learned Streetwise on the run.".into()
            }
            _ => "Mishap: Injured but recovered after medical leave; forced out of career.".into(),
        };
        session.current_career = None;
    } else {
        session.benefit_rolls_earned += 1;
        // 4. Career Event (2D6)
        let ev_dice = roll_nd6(rng, 2);
        let ev_roll = ev_dice[0] + ev_dice[1];
        event = match ev_roll {
            2..=4 => {
                session.rivals.push(format!("{} Competitor", active_spec.name));
                "Event: Crossed paths with a dangerous Rival during operations.".into()
            }
            5..=6 => {
                let bonus_sk = active_spec.specialist_skills[rng.random_range(0..6)];
                skills_gained.push(gain_skill(&mut session.skills, bonus_sk));
                format!("Event: Special assignment honed {bonus_sk}.")
            }
            7..=8 => {
                session.allies.push(format!("{} Contact", active_spec.name));
                "Event: Made a reliable Ally in the sector.".into()
            }
            9..=10 => {
                session.ship_shares += 1;
                "Event: Awarded +1 Ship Share for exemplary service.".into()
            }
            _ => {
                session.benefit_rolls_earned += 1;
                "Event: Exceptional commendation! +1 bonus Benefit roll.".into()
            }
        };

        // 5. Advancement Roll
        let a_dice = roll_nd6(rng, 2);
        let a_total =
            i32::from(a_dice[0]) + i32::from(a_dice[1]) + session.upp.dm(active_spec.adv_stat);
        if a_total >= active_spec.adv_target {
            advanced = true;
            session.current_rank = (session.current_rank + 1).min(6);
            if matches!(session.current_rank, 1 | 3 | 5) {
                session.benefit_rolls_earned += 1;
            }
            let adv_sk = active_spec.specialist_skills[rng.random_range(0..6)];
            skills_gained.push(gain_skill(&mut session.skills, adv_sk));
        }
    }

    session.terms_completed += 1;
    session.age += 4;

    // 6. Aging Roll (Core 2022 p. 47: at end of 4th term / age 34+, 2D6 - terms_completed; synthetics exempt)
    let ignores_aging = find_species(&session.species)
        .map(|s| s.ignores_aging)
        .unwrap_or(false);
    let aging_note = if session.age >= 34 && !ignores_aging {
        let ag_dice = roll_nd6(rng, 2);
        let ag_total = i32::from(ag_dice[0]) + i32::from(ag_dice[1]) - i32::from(session.terms_completed);
        if ag_total <= 0 {
            session.upp.str_val = (session.upp.str_val - 1).max(1);
            session.upp.dex_val = (session.upp.dex_val - 1).max(1);
            Some(format!("Aging roll {ag_total}: STR -1, DEX -1"))
        } else {
            Some(format!("Aging roll {ag_total}: no physical decline"))
        }
    } else {
        None
    };

    let career_label = match &session.current_assignment {
        Some(a) => format!("{} ({a})", active_spec.name),
        None => active_spec.name.to_string(),
    };

    let entry = LifepathLogEntry {
        term: term_num,
        age: session.age,
        career: career_label,
        qualified,
        survived,
        advanced,
        rank: session.current_rank,
        skills_gained,
        event,
        aging_note,
    };
    session.log.push(entry.clone());
    entry
}

/// Muster out: roll remaining benefit rolls (up to 3 Cash rolls, remainder on Benefits/Ship Shares)
/// and compute Core 2022 Pension (5+ terms -> Cr10,000 + Cr2,000 per extra term/year).
pub fn muster_out<R: Rng + ?Sized>(
    rng: &mut R,
    session: &mut LifepathSession,
    cash_rolls_requested: u32,
) {
    if session.mustered_out {
        return;
    }
    let career = session.current_career.unwrap_or(Career::Drifter);
    let spec = career_spec(career);
    let total_rolls = session.benefit_rolls_earned.max(1);
    let cash_rolls = cash_rolls_requested.min(3).min(total_rolls);
    let benefit_rolls = total_rolls.saturating_sub(cash_rolls);

    let has_gambler_dm = if session.skills.get("Gambler").copied().unwrap_or(-1) >= 1 {
        1
    } else {
        0
    };
    let rank_dm = if session.current_rank >= 5 { 1 } else { 0 };

    for _ in 0..cash_rolls {
        let idx = ((roll_nd6(rng, 1)[0] as usize) - 1 + has_gambler_dm).min(6);
        session.cash_cr += spec.cash_table[idx];
    }

    for _ in 0..benefit_rolls {
        let roll = ((roll_nd6(rng, 1)[0] as usize) + rank_dm).min(7);
        match roll {
            1 => session.upp.int_val += 1,
            2 => session.upp.edu_val += 1,
            3 | 4 => session.ship_shares += 1,
            5 => session.ship_shares += 2,
            6 => {
                if spec.has_ship_benefit {
                    session.career_ship_benefits += 1;
                } else {
                    session.upp.soc_val += 1;
                }
            }
            _ => {
                if spec.has_ship_benefit {
                    session.career_ship_benefits += 1;
                } else {
                    session.ship_shares += 2;
                }
            }
        }
    }

    if session.terms_completed >= 5 {
        session.pension_cr = 10_000 + i64::from(session.terms_completed - 5) * 2_000;
    }
    session.mustered_out = true;
}

/// Convert a finalized LifepathSession into a live `Character` record ready for Postgres.
pub fn finalize_lifepath_character(
    campaign_id: Uuid,
    kind: CharacterKind,
    session: &LifepathSession,
    salary_cr: i64,
    station: Option<String>,
) -> Character {
    let mut notes = format!(
        "Generated via MgT2e Lifepath ({}, {} terms, Age {}). Cash Cr{}, Ship Shares {}, Career Ship Benefits {}, Pension Cr{}/yr.",
        session.species,
        session.terms_completed,
        session.age,
        session.cash_cr,
        session.ship_shares,
        session.career_ship_benefits,
        session.pension_cr
    );
    if !session.species_traits.is_empty() {
        notes.push_str(&format!(" Species traits: {}", session.species_traits));
    }
    if let Some(pre) = &session.pre_career_note {
        notes.push_str(&format!(" Pre-Career: {pre}"));
    }
    if !session.allies.is_empty() {
        notes.push_str(&format!(" Allies: {}.", session.allies.join(", ")));
    }
    if !session.rivals.is_empty() {
        notes.push_str(&format!(" Rivals: {}.", session.rivals.join(", ")));
    }

    Character {
        id: Uuid::new_v4(),
        campaign_id,
        kind,
        name: session.name.clone(),
        species: session.species.clone(),
        age: session.age,
        upp_max: session.upp.clone(),
        upp_current: session.upp.clone(),
        skills: session.skills.clone(),
        augments: vec![],
        skill_xp: BTreeMap::new(),
        characteristic_xp: BTreeMap::new(),
        general_xp: 0,
        salary_cr,
        loyalty: 0,
        assigned_ship_id: None,
        assigned_station: station,
        notes,
    }
}
