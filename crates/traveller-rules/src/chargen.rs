use std::collections::BTreeMap;

use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::character::{
    characteristic_dm, skill_base_name, skill_specializations, Character, CharacterKind,
    Characteristic, Upp,
};
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

/// Core 2022 p. 16 full Background Skills list (choose EDU DM + 3 at Level 0 — NO specialisations at Level 0!).
pub fn all_background_skills() -> &'static [&'static str] {
    &[
        "Admin",
        "Animals",
        "Art",
        "Athletics",
        "Carouse",
        "Drive",
        "Electronics",
        "Flyer",
        "Language",
        "Mechanic",
        "Medic",
        "Profession",
        "Science",
        "Seafarer",
        "Streetwise",
        "Survival",
        "Vacc Suit",
    ]
}

/// Core 2022 p. 17 University Minor Skills (starts at Level 0 — unspecialised base skills; gains specialisation only on graduation to Level 1!).
pub fn all_university_minor_skills() -> &'static [&'static str] {
    &[
        "Admin",
        "Advocate",
        "Animals",
        "Art",
        "Astrogation",
        "Broker",
        "Electronics",
        "Engineer",
        "Language",
        "Medic",
        "Navigation",
        "Profession",
        "Science",
    ]
}

/// Core 2022 p. 17 University Major Study Skills (starts at Level 1, so specialised skills carry a specialisation; +1 on graduation).
pub fn all_university_skills() -> &'static [&'static str] {
    &[
        "Admin",
        "Advocate",
        "Animals (Training)",
        "Art (Visual Media)",
        "Astrogation",
        "Broker",
        "Electronics (Comms)",
        "Electronics (Computers)",
        "Electronics (Sensors)",
        "Engineer (Power)",
        "Language (Galanglic)",
        "Medic",
        "Navigation",
        "Profession (Belter)",
        "Science (Economics)",
        "Science (History)",
        "Science (Physics)",
        "Science (Robotics)",
        "Science (Xenology)",
    ]
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillPackageOption {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub skills: &'static [&'static str],
}

/// Core 2022 p. 51 Starting Skill Packages (pick 2 skills at Level 1 after mustering out).
pub fn all_skill_packages() -> &'static [SkillPackageOption] {
    &[
        SkillPackageOption {
            id: "trader",
            name: "Free Trader Package",
            description: "Essential mercantile & ship-operations skills for independent starship captains.",
            skills: &["Broker", "Pilot (Spacecraft)", "Astrogation", "Persuade", "Steward", "Streetwise", "Vacc Suit", "Engineer (J-Drive)"],
        },
        SkillPackageOption {
            id: "explorer",
            name: "Scout & Explorer Package",
            description: "Deep-space survey, sensors, survival, and frontier navigation.",
            skills: &["Pilot (Spacecraft)", "Astrogation", "Electronics (sensors)", "Survival", "Recon", "Medic", "Vacc Suit", "Mechanic"],
        },
        SkillPackageOption {
            id: "starship_crew",
            name: "Starship Operations Package",
            description: "Bridge, engineering, gunnery, and shipboard damage control.",
            skills: &["Pilot (Spacecraft)", "Astrogation", "Engineer (M-Drive)", "Engineer (Power)", "Gunner (Turret)", "Electronics (sensors)", "Mechanic", "Vacc Suit"],
        },
        SkillPackageOption {
            id: "mercenary",
            name: "Mercenary & Security Package",
            description: "Personal combat, tactics, heavy weapons, and field trauma care.",
            skills: &["Gun Combat (Slug)", "Gun Combat (Energy)", "Melee (Blade)", "Tactics (Military)", "Recon", "Stealth", "Medic", "Leadership"],
        },
        SkillPackageOption {
            id: "investigator",
            name: "Patron & Investigator Package",
            description: "Espionage, diplomacy, investigation, and underworld contacts.",
            skills: &["Investigate", "Persuade", "Deception", "Streetwise", "Diplomat", "Electronics (computers)", "Advocate", "Stealth"],
        },
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifepathStage {
    AssignStats,
    BackgroundSkills,
    PreCareer,
    ChooseCareer,
    FailedQualification,
    MusterOut,
    SkillPackage,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MilitaryBranch {
    Army,
    Marine,
    Navy,
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
    Officer,
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
    #[serde(default = "default_stage")]
    pub stage: LifepathStage,
    /// Raw 6x 2D6 pool rolled in Phase 1 (before species modifiers), freely assignable to STR..SOC.
    #[serde(default)]
    pub rolled_pool: Vec<i32>,
    #[serde(default)]
    pub stats_locked: bool,
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
    pub auto_qualify_career: Option<Career>,
    #[serde(default)]
    pub auto_commission_career: Option<Career>,
    #[serde(default)]
    pub qualification_bonus_dm: i32,
    #[serde(default)]
    pub draft_used: bool,
    #[serde(default)]
    pub pending_failed_career: Option<Career>,
    #[serde(default)]
    pub is_officer: bool,
    #[serde(default)]
    pub must_leave_career: bool,
    #[serde(default)]
    pub must_stay_career: bool,
    #[serde(default)]
    pub terms_in_current_career: u16,
    #[serde(default)]
    pub careers_entered: Vec<Career>,
    pub current_career: Option<Career>,
    #[serde(default)]
    pub current_assignment: Option<String>,
    pub current_rank: u8,
    pub benefit_rolls_earned: u32,
    pub cash_cr: i64,
    pub ship_shares: u32,
    pub career_ship_benefits: u32,
    pub pension_cr: i64,
    #[serde(default)]
    pub material_benefits: Vec<String>,
    #[serde(default)]
    pub skill_package_applied: bool,
    pub allies: Vec<String>,
    pub rivals: Vec<String>,
    pub mustered_out: bool,
    pub log: Vec<LifepathLogEntry>,
}

fn default_stage() -> LifepathStage {
    LifepathStage::ChooseCareer
}

fn apply_pool_with_species(pool: &[i32], sp: Option<&SpeciesSpec>, psi_val: Option<i32>) -> Upp {
    let clamp_stat = |v: i32| v.clamp(1, 15);
    let p = |idx: usize| pool.get(idx).copied().unwrap_or(7);
    Upp {
        str_val: clamp_stat(p(0) + sp.map(|s| s.str_mod).unwrap_or(0)),
        dex_val: clamp_stat(p(1) + sp.map(|s| s.dex_mod).unwrap_or(0)),
        end_val: clamp_stat(p(2) + sp.map(|s| s.end_mod).unwrap_or(0)),
        int_val: clamp_stat(p(3) + sp.map(|s| s.int_mod).unwrap_or(0)),
        edu_val: clamp_stat(p(4) + sp.map(|s| s.edu_mod).unwrap_or(0)),
        soc_val: clamp_stat(p(5) + sp.map(|s| s.soc_mod).unwrap_or(0)),
        psi_val,
    }
}

/// Start a new MgT2e lifepath character by rolling six 2D6 values, applying authentic species
/// modifiers/traits, and preparing for characteristic assignment & background skills (Core 2022 p. 10–16).
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

    let rolled_pool = vec![r2(), r2(), r2(), r2(), r2(), r2()];
    let psi_val = if sp.map(|s| s.psi_capable).unwrap_or(false) {
        Some(r2())
    } else {
        None
    };
    let upp = apply_pool_with_species(&rolled_pool, sp, psi_val);

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
        // Core 2022 p. 16 & p. 62: Level 0 background skills NEVER have specialisations.
        let base = skill_base_name(&sk);
        if !base.is_empty() {
            skills.entry(base.to_string()).or_insert(0);
        }
    }
    if canonical_species == "Bwap" {
        skills.entry("Admin".to_string()).or_insert(0);
    }

    LifepathSession {
        id: Uuid::new_v4(),
        name,
        species: canonical_species,
        species_traits,
        stage: if background_skills_chosen {
            LifepathStage::PreCareer
        } else {
            LifepathStage::AssignStats
        },
        rolled_pool,
        stats_locked: background_skills_chosen,
        age: 18,
        terms_completed: 0,
        upp,
        skills,
        background_skill_slots: bg_limit as u32,
        background_skills_chosen,
        pre_career_done: false,
        pre_career_note: None,
        auto_qualify_career: None,
        auto_commission_career: None,
        qualification_bonus_dm: 0,
        draft_used: false,
        pending_failed_career: None,
        is_officer: false,
        must_leave_career: false,
        must_stay_career: false,
        terms_in_current_career: 0,
        careers_entered: vec![],
        current_career: None,
        current_assignment: None,
        current_rank: 0,
        benefit_rolls_earned: 0,
        cash_cr: 0,
        ship_shares: 0,
        career_ship_benefits: 0,
        pension_cr: 0,
        material_benefits: vec![],
        skill_package_applied: false,
        allies: vec![],
        rivals: vec![],
        mustered_out: false,
        log: vec![],
    }
}

/// Core 2022 p. 10: Assign the 6 rolled 2D6 characteristic values in any order the player chooses.
/// Validates that `ordered_pool` is an exact permutation of `session.rolled_pool` (no stat cheating!).
pub fn assign_characteristics(
    session: &mut LifepathSession,
    ordered_pool: Vec<i32>,
    lock: bool,
) -> anyhow::Result<()> {
    if session.stats_locked || session.terms_completed > 0 || session.pre_career_done {
        anyhow::bail!("Characteristics are already locked.");
    }
    if ordered_pool.len() != 6 || session.rolled_pool.len() != 6 {
        anyhow::bail!("Must assign all 6 rolled characteristic values.");
    }
    let mut a = session.rolled_pool.clone();
    let mut b = ordered_pool.clone();
    a.sort_unstable();
    b.sort_unstable();
    if a != b {
        anyhow::bail!("Assigned characteristics must be an exact permutation of your 6 rolled 2D6 values.");
    }
    session.rolled_pool = ordered_pool;
    let sp = find_species(&session.species);
    session.upp = apply_pool_with_species(&session.rolled_pool, sp, session.upp.psi_val);
    let bg_limit = (characteristic_dm(session.upp.edu_val) + 3).max(1) as u32;
    session.background_skill_slots = bg_limit;
    if lock {
        session.stats_locked = true;
        session.stage = LifepathStage::BackgroundSkills;
        // Reset default skills to empty so player picks their exact background skills cleanly
        session.skills.clear();
        if session.species == "Bwap" {
            session.skills.entry("Admin".to_string()).or_insert(0);
        }
    }
    Ok(())
}

/// Set the player's chosen Core 2022 background skills at Level 0 (allowed before Pre-Career / Term 1).
/// Per Core 2022 p. 62, Level 0 skills never have specialisations; any `(Spec)` suffix is stripped.
pub fn set_background_skills(session: &mut LifepathSession, chosen: Vec<String>) {
    if session.terms_completed > 0 || session.pre_career_done {
        return;
    }
    let limit = session.background_skill_slots.max(1) as usize;
    session.skills.clear();
    for sk in chosen.into_iter().take(limit) {
        let base = skill_base_name(&sk);
        if !base.is_empty() {
            session.skills.entry(base.to_string()).or_insert(0);
        }
    }
    if session.species == "Bwap" {
        session.skills.entry("Admin".to_string()).or_insert(0);
    }
    session.stats_locked = true;
    session.background_skills_chosen = true;
    session.stage = LifepathStage::PreCareer;
}

/// Run Core 2022 Pre-Career Education (University with player's 2 chosen skills,
/// Military Academy with chosen branch & 3 promoted skills, or Skip) before Term 1.
#[allow(clippy::too_many_arguments)]
pub fn run_pre_career_detailed<R: Rng + ?Sized>(
    rng: &mut R,
    session: &mut LifepathSession,
    choice: PreCareerChoice,
    uni_skill_0: Option<String>,
    uni_skill_1: Option<String>,
    academy_branch: Option<MilitaryBranch>,
    academy_focus_skills: Vec<String>,
) -> String {
    if session.pre_career_done || session.terms_completed > 0 {
        return "Pre-career education is only available before regular adult careers.".into();
    }
    session.stats_locked = true;
    session.background_skills_chosen = true;
    session.pre_career_done = true;
    session.stage = LifepathStage::ChooseCareer;

    match choice {
        PreCareerChoice::None => {
            let note = "Skipped pre-career education and entered adult careers at Age 18.".to_string();
            session.pre_career_note = Some(note.clone());
            note
        }
        PreCareerChoice::University => {
            let soc_dm = if session.upp.soc_val >= 9 { 1 } else { 0 };
            let d = roll_nd6(rng, 2);
            let entry_roll = i32::from(d[0]) + i32::from(d[1]) + session.upp.dm(Characteristic::Edu) + soc_dm;
            if entry_roll < 7 {
                let note = format!(
                    "University admission denied (rolled {entry_roll} vs EDU 7+); proceeding to adult careers at Age 18."
                );
                session.pre_career_note = Some(note.clone());
                return note;
            }
            session.age += 4;
            session.terms_completed += 1;
            session.upp.edu_val = (session.upp.edu_val + 1).min(15);

            // Core 2022 p. 17 & p. 62:
            // - Minor Subject starts at Level 0 (NO specialisation at Level 0!).
            //   If the student graduates and raises it to Level 1, it takes the chosen specialisation
            //   (if any) or first canonical specialisation for specialised skills.
            // - Major Subject starts at Level 1 (specialised if applicable).
            let raw_s0 = uni_skill_0
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "Admin".to_string());
            let s0_base = skill_base_name(&raw_s0).to_string();
            let s0_grad_target = if raw_s0.contains('(') {
                raw_s0.clone()
            } else {
                let specs = skill_specializations(&s0_base);
                if specs.is_empty() {
                    s0_base.clone()
                } else {
                    format!("{s0_base} ({})", specs[0])
                }
            };
            let s1 = uni_skill_1
                .filter(|s| !s.trim().is_empty() && s != &s0_base && s != &raw_s0)
                .unwrap_or_else(|| "Electronics (Computers)".to_string());
            session.skills.entry(s0_base.clone()).or_insert(0);
            let s1_entry = session.skills.entry(s1.clone()).or_insert(0);
            if *s1_entry < 1 {
                *s1_entry = 1;
            } else {
                *s1_entry += 1;
            }

            // Pre-Career Event (2D6)
            let ev_d = roll_nd6(rng, 2);
            let ev_roll = ev_d[0] + ev_d[1];
            let ev_text = match ev_roll {
                2..=4 => {
                    session.rivals.push("University Rival".into());
                    "Campus rivalry left you with a lasting academic Rival."
                }
                5..=7 => {
                    session.allies.push("University Mentor".into());
                    "Formed a strong bond with a well-connected Ally."
                }
                8..=10 => {
                    gain_skill_level1(&mut session.skills, "Carouse");
                    "Active social circle honed Carouse."
                }
                _ => {
                    session.upp.soc_val = (session.upp.soc_val + 1).min(15);
                    "Published acclaimed research — SOC +1!"
                }
            };

            // Graduation Roll: INT 7+ (Honours 11+)
            let gd = roll_nd6(rng, 2);
            let grad = i32::from(gd[0]) + i32::from(gd[1]) + session.upp.dm(Characteristic::Int);
            let (note, minor_display) = if grad >= 11 {
                session.upp.edu_val = (session.upp.edu_val + 2).min(15);
                let m_label = gain_skill_level1(&mut session.skills, &s0_grad_target);
                *session.skills.entry(s1.clone()).or_insert(1) += 1;
                session.qualification_bonus_dm += 2;
                let n = format!(
                    "University Graduated with HONOURS (roll {grad}/11+)! EDU +3 total, {m_label}, {s1}-{}, +2 DM to first career qualification. Event: {ev_text}",
                    session.skills.get(&s1).copied().unwrap_or(2)
                );
                (n, m_label)
            } else if grad >= 7 {
                session.upp.edu_val = (session.upp.edu_val + 1).min(15);
                let m_label = gain_skill_level1(&mut session.skills, &s0_grad_target);
                *session.skills.entry(s1.clone()).or_insert(1) += 1;
                session.qualification_bonus_dm += 1;
                let n = format!(
                    "University Graduated (roll {grad}/7+)! EDU +2 total, {m_label}, {s1}-{}, +1 DM to first career qualification. Event: {ev_text}",
                    session.skills.get(&s1).copied().unwrap_or(2)
                );
                (n, m_label)
            } else {
                let m_label = format!("{s0_base}-{}", session.skills.get(&s0_base).copied().unwrap_or(0));
                let n = format!(
                    "Attended University (did not graduate, roll {grad}/7+). Kept EDU +1, {m_label}, {s1}-{}. Event: {ev_text}",
                    session.skills.get(&s1).copied().unwrap_or(1)
                );
                (n, m_label)
            };
            session.pre_career_note = Some(note.clone());
            session.log.push(LifepathLogEntry {
                term: 1,
                age: session.age,
                career: "University (Pre-Career)".into(),
                qualified: true,
                survived: grad >= 7,
                advanced: grad >= 11,
                rank: 0,
                skills_gained: vec![
                    minor_display,
                    format!("{s1}-{}", session.skills.get(&s1).copied().unwrap_or(1)),
                ],
                event: note.clone(),
                aging_note: None,
            });
            note
        }
        PreCareerChoice::MilitaryAcademy => {
            let branch = academy_branch.unwrap_or(MilitaryBranch::Navy);
            let (branch_name, branch_career, entry_stat, entry_target, grad_stat, soc_threshold) = match branch {
                MilitaryBranch::Army => ("Army Academy", Career::Army, Characteristic::End, 8, Characteristic::Int, 7),
                MilitaryBranch::Marine => ("Marine Academy", Career::Marine, Characteristic::End, 9, Characteristic::End, 8),
                MilitaryBranch::Navy => ("Naval Academy", Career::Navy, Characteristic::Int, 9, Characteristic::Int, 8),
            };
            let soc_dm = if session.upp.soc_val >= soc_threshold { 1 } else { 0 };
            let d = roll_nd6(rng, 2);
            let entry_roll = i32::from(d[0]) + i32::from(d[1]) + session.upp.dm(entry_stat) + soc_dm;
            if entry_roll < entry_target {
                let note = format!(
                    "{branch_name} admission failed (rolled {entry_roll} vs {entry_stat:?} {entry_target}+); proceeding to adult careers at Age 18."
                );
                session.pre_career_note = Some(note.clone());
                return note;
            }
            session.age += 4;
            session.terms_completed += 1;
            let bspec = career_spec(branch_career);
            // Entry benefit: ALL service skills of that military branch at Level 0 (unspecialised per Core 2022 p. 62)!
            let mut gained = Vec::new();
            let mut seen_bt = std::collections::BTreeSet::new();
            for sk in bspec.service_skills {
                let base = skill_base_name(sk);
                if seen_bt.insert(base) {
                    session.skills.entry(base.to_string()).or_insert(0);
                    gained.push(format!("{base}-0"));
                }
            }

            let gd = roll_nd6(rng, 2);
            let grad = i32::from(gd[0]) + i32::from(gd[1]) + session.upp.dm(grad_stat);
            let focus: Vec<String> = if academy_focus_skills.is_empty() {
                bspec.service_skills.iter().take(3).map(|s| (*s).to_string()).collect()
            } else {
                academy_focus_skills.into_iter().take(3).collect()
            };

            let note = if grad >= 11 {
                for sk in &focus {
                    gained.push(gain_skill_level1(&mut session.skills, sk));
                }
                session.upp.soc_val = (session.upp.soc_val + 1).min(15);
                gained.push(gain_skill_level1(&mut session.skills, "Leadership"));
                session.auto_qualify_career = Some(branch_career);
                session.auto_commission_career = Some(branch_career);
                format!(
                    "{branch_name} Graduated with HONOURS (roll {grad}/11+)! SOC +1, Leadership-1, raised {}, auto-qualifies & auto-commissions as Officer Rank 1 in {}!",
                    focus.join(", "),
                    bspec.name
                )
            } else if grad >= 8 {
                for sk in &focus {
                    gained.push(gain_skill_level1(&mut session.skills, sk));
                }
                session.upp.edu_val = (session.upp.edu_val + 1).min(15);
                session.auto_qualify_career = Some(branch_career);
                format!(
                    "{branch_name} Graduated (roll {grad}/8+)! EDU +1, raised {}, automatic qualification into {}!",
                    focus.join(", "),
                    bspec.name
                )
            } else {
                format!(
                    "Attended {branch_name} (did not graduate, roll {grad}/8+). Retained Basic Training skills at level 0."
                )
            };
            session.pre_career_note = Some(note.clone());
            session.log.push(LifepathLogEntry {
                term: 1,
                age: session.age,
                career: format!("{branch_name} (Pre-Career)"),
                qualified: true,
                survived: grad >= 8,
                advanced: grad >= 11,
                rank: 0,
                skills_gained: gained,
                event: note.clone(),
                aging_note: None,
            });
            note
        }
    }
}

pub fn run_pre_career<R: Rng + ?Sized>(
    rng: &mut R,
    session: &mut LifepathSession,
    choice: PreCareerChoice,
) -> String {
    run_pre_career_detailed(rng, session, choice, None, None, None, vec![])
}

/// Core 2022 p. 20: Career/University/Event/Rank skill gains grant Level 1 if you do NOT yet have
/// the skill, or +1 if you already possess it at level 0 or higher!
fn gain_skill_level1(skills: &mut BTreeMap<String, i32>, name: &str) -> String {
    let entry = skills.entry(name.to_string()).or_insert(-1);
    if *entry < 0 {
        *entry = 1;
    } else {
        *entry += 1;
    }
    format!("{name}-{}", *entry)
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
        other => gain_skill_level1(skills, other),
    }
}

fn officer_skills_for(career: Career) -> [&'static str; 6] {
    match career {
        Career::Navy => ["Leadership", "Electronics (sensors)", "Pilot (Spacecraft)", "Tactics (Naval)", "Admin", "Melee (Blade)"],
        Career::Army => ["Tactics (Military)", "Leadership", "Advocate", "Diplomat", "Electronics (comms)", "Admin"],
        Career::Marine => ["Electronics (comms)", "Tactics (Military)", "Admin", "Advocate", "Stealth", "Leadership"],
        Career::Merchant => ["Broker", "Admin", "Persuade", "Diplomat", "Pilot (Spacecraft)", "Leadership"],
        _ => ["Leadership", "Admin", "Diplomat", "Advocate", "Persuade", "Tactics (Military)"],
    }
}

fn rank_bonus_skill(career: Career, rank: u8) -> Option<&'static str> {
    match (career, rank) {
        (Career::Agent, 1) => Some("Streetwise"),
        (Career::Agent, 3) => Some("Investigate"),
        (Career::Army, 1) => Some("Gun Combat (Slug)"),
        (Career::Army, 2) => Some("Leadership"),
        (Career::Citizen, 2) => Some("Admin"),
        (Career::Drifter, 1) => Some("Survival"),
        (Career::Entertainer, 1) => Some("Art (Performer)"),
        (Career::Marine, 1) => Some("Gun Combat (Energy)"),
        (Career::Marine, 2) => Some("Tactics (Military)"),
        (Career::Merchant, 1) => Some("Persuade"),
        (Career::Merchant, 3) => Some("Broker"),
        (Career::Navy, 1) => Some("Melee (Blade)"),
        (Career::Navy, 2) => Some("Leadership"),
        (Career::Noble, 1) => Some("Admin"),
        (Career::Noble, 3) => Some("Diplomat"),
        (Career::Rogue, 1) => Some("Deception"),
        (Career::Rogue, 3) => Some("Streetwise"),
        (Career::Scholar, 1) => Some("Investigate"),
        (Career::Scholar, 3) => Some("Science (Physics)"),
        (Career::Scout, 1) => Some("Vacc Suit"),
        (Career::Scout, 3) => Some("Pilot (Spacecraft)"),
        _ => None,
    }
}

/// Core 2022 Draft table (1D6) when qualification fails and the player chooses Draft over Drifter.
pub fn draft_career<R: Rng + ?Sized>(rng: &mut R) -> Career {
    match roll_nd6(rng, 1)[0] {
        1 => Career::Navy,
        2 => Career::Army,
        3 => Career::Marine,
        4 => Career::Merchant,
        5 => Career::Scout,
        _ => Career::Agent,
    }
}

/// Apply the player's chosen Starting Skill Package (2 skills set to at least Level 1, Core 2022 p. 51).
pub fn apply_skill_package(session: &mut LifepathSession, chosen_skills: Vec<String>) {
    if session.skill_package_applied {
        return;
    }
    for sk in chosen_skills.into_iter().take(2) {
        let entry = session.skills.entry(sk).or_insert(0);
        if *entry < 1 {
            *entry = 1;
        }
    }
    session.skill_package_applied = true;
    session.stage = LifepathStage::Ready;
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

/// Full MgT2e Career Term with Assignment, Skill Table selection, optional Draft choice,
/// Officer Commission attempt, Rank Bonus skills, and authentic Level 1 skill gains!
#[allow(clippy::too_many_arguments)]
pub fn run_lifepath_term_full<R: Rng + ?Sized>(
    rng: &mut R,
    session: &mut LifepathSession,
    career: Career,
    assignment: Option<String>,
    skill_table: SkillTableChoice,
    basic_training_skill: Option<String>,
    attempt_commission: bool,
    fallback_on_fail: Option<&str>, // "draft" | "drifter" | None (auto-drifter)
) -> LifepathLogEntry {
    let spec = career_spec(career);
    let term_num = session.terms_completed + 1;
    let mut skills_gained = Vec::new();
    session.stats_locked = true;
    session.background_skills_chosen = true;
    session.pre_career_done = true;
    session.stage = LifepathStage::ChooseCareer;

    let mut qualified = true;
    let mut was_drafted = false;
    let entering_new = session.current_career != Some(career) || session.must_leave_career;
    let actual_career = if entering_new {
        let auto_ok = session.auto_qualify_career == Some(career);
        let pass = if auto_ok {
            session.auto_qualify_career = None;
            true
        } else {
            let d = roll_nd6(rng, 2);
            let prev_careers_pen = session.careers_entered.len() as i32;
            let roll = i32::from(d[0])
                + i32::from(d[1])
                + session.upp.dm(spec.qual_stat)
                + session.qualification_bonus_dm
                - prev_careers_pen;
            session.qualification_bonus_dm = 0;
            roll >= spec.qual_target
        };

        if pass {
            session.current_career = Some(career);
            session.current_assignment = assignment.or_else(|| Some(spec.assignments[0].to_string()));
            session.current_rank = 0;
            session.terms_in_current_career = 0;
            session.must_leave_career = false;
            session.must_stay_career = false;
            session.is_officer = false;
            if session.auto_commission_career == Some(career) {
                session.auto_commission_career = None;
                session.is_officer = true;
                session.current_rank = 1;
                skills_gained.push(gain_skill_level1(&mut session.skills, "Leadership"));
            }
            let first_adult_career = session.careers_entered.is_empty();
            session.careers_entered.push(career);
            // Basic training (Core 2022 p. 20 & p. 62): Level 0 skills NEVER have specialisations!
            // First adult career grants all service skills at Level 0; later careers pick 1.
            if first_adult_career {
                let mut seen_bt = std::collections::BTreeSet::new();
                for sk in spec.service_skills {
                    let base = skill_base_name(sk);
                    if seen_bt.insert(base) {
                        session.skills.entry(base.to_string()).or_insert(0);
                        skills_gained.push(format!("{base}-0 (Basic)"));
                    }
                }
            } else {
                let chosen_bt = basic_training_skill
                    .as_deref()
                    .map(skill_base_name)
                    .filter(|b| spec.service_skills.iter().any(|s| skill_base_name(s).eq_ignore_ascii_case(b)))
                    .unwrap_or_else(|| skill_base_name(spec.service_skills[rng.random_range(0..6)]));
                session.skills.entry(chosen_bt.to_string()).or_insert(0);
                skills_gained.push(format!("{chosen_bt}-0 (Basic)"));
            }
            career
        } else {
            qualified = false;
            let chosen_fallback = match fallback_on_fail {
                Some("draft") if !session.draft_used => {
                    session.draft_used = true;
                    was_drafted = true;
                    draft_career(rng)
                }
                _ => Career::Drifter,
            };
            let fspec = career_spec(chosen_fallback);
            session.current_career = Some(chosen_fallback);
            session.current_assignment = Some(fspec.assignments[0].to_string());
            session.current_rank = 0;
            session.terms_in_current_career = 0;
            session.must_leave_career = false;
            session.must_stay_career = false;
            session.is_officer = false;
            let first_adult_career = session.careers_entered.is_empty();
            session.careers_entered.push(chosen_fallback);
            if first_adult_career {
                let mut seen_bt = std::collections::BTreeSet::new();
                for sk in fspec.service_skills {
                    let base = skill_base_name(sk);
                    if seen_bt.insert(base) {
                        session.skills.entry(base.to_string()).or_insert(0);
                        skills_gained.push(format!("{base}-0 (Basic)"));
                    }
                }
            } else {
                let base = skill_base_name(fspec.service_skills[0]);
                session.skills.entry(base.to_string()).or_insert(0);
                skills_gained.push(format!("{base}-0 (Basic)"));
            }
            chosen_fallback
        }
    } else {
        if let Some(a) = assignment {
            session.current_assignment = Some(a);
        }
        career
    };

    let active_spec = career_spec(actual_career);

    // 2. Term Skill Roll (Level 1 if new, +1 if existing!)
    let effective_table = match skill_table {
        SkillTableChoice::AdvancedEdu if session.upp.edu_val < 8 => SkillTableChoice::Specialist,
        SkillTableChoice::Officer if !session.is_officer => SkillTableChoice::Specialist,
        other => other,
    };
    let officer_tbl = officer_skills_for(actual_career);
    let table: &[&str; 6] = match effective_table {
        SkillTableChoice::Service => &active_spec.service_skills,
        SkillTableChoice::Specialist => &active_spec.specialist_skills,
        SkillTableChoice::Personal => &active_spec.personal_dev,
        SkillTableChoice::AdvancedEdu => &active_spec.advanced_edu_skills,
        SkillTableChoice::Officer => &officer_tbl,
    };
    let sk_idx = rng.random_range(0..6);
    skills_gained.push(gain_skill_or_stat(&mut session.upp, &mut session.skills, table[sk_idx]));

    // 3. Survival Roll (Core 2022: natural 2 ALWAYS fails!)
    let s_dice = roll_nd6(rng, 2);
    let s_raw = s_dice[0] + s_dice[1];
    let s_total = i32::from(s_raw) + session.upp.dm(active_spec.surv_stat);
    let survived = s_raw != 2 && s_total >= active_spec.surv_target;

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
                skills_gained.push(gain_skill_level1(&mut session.skills, "Streetwise"));
                "Mishap: Whistleblower incident — learned Streetwise on the run.".into()
            }
            _ => "Mishap: Injured but recovered after medical leave; forced out of career.".into(),
        };
        session.current_career = None;
        session.must_leave_career = true;
        session.must_stay_career = false;
    } else {
        session.benefit_rolls_earned += 1;
        session.terms_in_current_career += 1;
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
                skills_gained.push(gain_skill_level1(&mut session.skills, bonus_sk));
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

        // 4b. Optional Officer Commission (Army / Marine / Navy, Core 2022 p. 22)
        if attempt_commission
            && !session.is_officer
            && !was_drafted
            && matches!(actual_career, Career::Army | Career::Marine | Career::Navy)
        {
            let cd = roll_nd6(rng, 2);
            let ctotal = i32::from(cd[0]) + i32::from(cd[1]) + session.upp.dm(Characteristic::Soc);
            if ctotal >= 8 {
                session.is_officer = true;
                session.current_rank = 1;
                skills_gained.push(gain_skill_level1(&mut session.skills, "Leadership"));
                skills_gained.push(format!("Commissioned Officer (SOC roll {ctotal}/8+)"));
            }
        }

        // 5. Advancement Roll (Core 2022 p. 22)
        let a_dice = roll_nd6(rng, 2);
        let a_raw = a_dice[0] + a_dice[1];
        let a_total = i32::from(a_raw) + session.upp.dm(active_spec.adv_stat);
        if a_total >= active_spec.adv_target {
            advanced = true;
            session.current_rank = (session.current_rank + 1).min(6);
            if matches!(session.current_rank, 1 | 3 | 5) {
                session.benefit_rolls_earned += 1;
            }
            if let Some(rank_sk) = rank_bonus_skill(actual_career, session.current_rank) {
                skills_gained.push(gain_skill_level1(&mut session.skills, rank_sk));
            }
            // Extra advancement bonus skill roll on chosen table
            let adv_sk = table[rng.random_range(0..6)];
            skills_gained.push(gain_skill_or_stat(&mut session.upp, &mut session.skills, adv_sk));
        }
        session.must_stay_career = a_raw == 12;
        if i32::from(a_raw) <= i32::from(session.terms_in_current_career) && a_raw != 12 {
            session.must_leave_career = true;
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

pub fn run_lifepath_term_detailed<R: Rng + ?Sized>(
    rng: &mut R,
    session: &mut LifepathSession,
    career: Career,
    assignment: Option<String>,
    skill_table: SkillTableChoice,
) -> LifepathLogEntry {
    run_lifepath_term_full(rng, session, career, assignment, skill_table, None, false, None)
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
            1 => {
                session.upp.int_val = (session.upp.int_val + 1).min(15);
                session.material_benefits.push("INT +1".into());
            }
            2 => {
                session.upp.edu_val = (session.upp.edu_val + 1).min(15);
                session.material_benefits.push("EDU +1".into());
            }
            3 => {
                session.ship_shares += 1;
                session.material_benefits.push("Ship Share ×1".into());
            }
            4 => {
                session.material_benefits.push("Personal Weapon (Autopistol or Blade)".into());
            }
            5 => {
                session.ship_shares += 2;
                session.material_benefits.push("Ship Shares ×2".into());
            }
            6 => {
                if spec.has_ship_benefit {
                    session.career_ship_benefits += 1;
                    session.material_benefits.push("Career Starship Benefit (25% mortgage reduction)".into());
                } else {
                    session.upp.soc_val = (session.upp.soc_val + 1).min(15);
                    session.material_benefits.push("SOC +1 & TAS Membership".into());
                }
            }
            _ => {
                if spec.has_ship_benefit {
                    session.career_ship_benefits += 1;
                    session.material_benefits.push("Career Starship Benefit (25% mortgage reduction)".into());
                } else {
                    session.ship_shares += 2;
                    session.material_benefits.push("Ship Shares ×2".into());
                }
            }
        }
    }

    if session.terms_completed >= 5 {
        session.pension_cr = 10_000 + i64::from(session.terms_completed - 5) * 2_000;
    }
    session.mustered_out = true;
    session.stage = LifepathStage::SkillPackage;
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
