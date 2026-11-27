use std::collections::BTreeMap;

use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::character::{characteristic_dm, Character, CharacterKind, Characteristic, Upp};
use crate::dice::roll_nd6;

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

#[derive(Debug, Clone, Serialize)]
pub struct CareerSpec {
    pub career: Career,
    pub name: &'static str,
    pub qual_stat: Characteristic,
    pub qual_target: i32,
    pub surv_stat: Characteristic,
    pub surv_target: i32,
    pub adv_stat: Characteristic,
    pub adv_target: i32,
    pub service_skills: [&'static str; 6],
    pub specialist_skills: [&'static str; 6],
    pub cash_table: [i64; 7],
    pub has_ship_benefit: bool,
}

pub fn career_spec(career: Career) -> CareerSpec {
    match career {
        Career::Agent => CareerSpec {
            career,
            name: "Agent",
            qual_stat: Characteristic::Int,
            qual_target: 6,
            surv_stat: Characteristic::Int,
            surv_target: 6,
            adv_stat: Characteristic::Edu,
            adv_target: 6,
            service_skills: ["Streetwise", "Drive (Wheel)", "Investigate", "Flyer (Grav)", "Recon", "Gun Combat (Slug)"],
            specialist_skills: ["Investigate", "Recon", "Electronics (comms)", "Stealth", "Persuade", "Deception"],
            cash_table: [1_000, 2_000, 5_000, 7_500, 10_000, 25_000, 50_000],
            has_ship_benefit: false,
        },
        Career::Army => CareerSpec {
            career,
            name: "Army",
            qual_stat: Characteristic::End,
            qual_target: 5,
            surv_stat: Characteristic::End,
            surv_target: 6,
            adv_stat: Characteristic::Edu,
            adv_target: 6,
            service_skills: ["Drive (Track)", "Athletics (Endurance)", "Gun Combat (Slug)", "Recon", "Melee (Blade)", "Heavy Weapons (Man Portable)"],
            specialist_skills: ["Tactics (Military)", "Leadership", "Gun Combat (Energy)", "Medic", "Stealth", "Survival"],
            cash_table: [2_000, 5_000, 10_000, 10_000, 10_000, 20_000, 30_000],
            has_ship_benefit: false,
        },
        Career::Citizen => CareerSpec {
            career,
            name: "Citizen",
            qual_stat: Characteristic::Edu,
            qual_target: 5,
            surv_stat: Characteristic::Edu,
            surv_target: 5,
            adv_stat: Characteristic::Int,
            adv_target: 6,
            service_skills: ["Drive (Wheel)", "Flyer (Grav)", "Streetwise", "Melee (Unarmed)", "Steward", "Profession (Belter)"],
            specialist_skills: ["Admin", "Advocate", "Broker", "Electronics (computers)", "Mechanic", "Science (Economics)"],
            cash_table: [1_000, 5_000, 10_000, 10_000, 10_000, 50_000, 100_000],
            has_ship_benefit: false,
        },
        Career::Drifter => CareerSpec {
            career,
            name: "Drifter",
            qual_stat: Characteristic::End,
            qual_target: 2, // Automatic
            surv_stat: Characteristic::End,
            surv_target: 7,
            adv_stat: Characteristic::Int,
            adv_target: 7,
            service_skills: ["Athletics (Endurance)", "Melee (Unarmed)", "Recon", "Streetwise", "Stealth", "Survival"],
            specialist_skills: ["Vacc Suit", "Mechanic", "Jack-of-All-Trades", "Gun Combat (Slug)", "Navigation", "Carouse"],
            cash_table: [0, 0, 1_000, 2_000, 3_000, 4_000, 8_000],
            has_ship_benefit: false,
        },
        Career::Entertainer => CareerSpec {
            career,
            name: "Entertainer",
            qual_stat: Characteristic::Int,
            qual_target: 5,
            surv_stat: Characteristic::Int,
            surv_target: 5,
            adv_stat: Characteristic::Soc,
            adv_target: 6,
            service_skills: ["Art (Performer)", "Art (Visual Media)", "Carouse", "Deception", "Persuade", "Steward"],
            specialist_skills: ["Carouse", "Diplomat", "Gambler", "Investigate", "Language (Galanglic)", "Streetwise"],
            cash_table: [0, 0, 10_000, 10_000, 40_000, 40_000, 80_000],
            has_ship_benefit: false,
        },
        Career::Marine => CareerSpec {
            career,
            name: "Marine",
            qual_stat: Characteristic::End,
            qual_target: 6,
            surv_stat: Characteristic::End,
            surv_target: 6,
            adv_stat: Characteristic::Edu,
            adv_target: 6,
            service_skills: ["Athletics (Dexterity)", "Vacc Suit", "Tactics (Military)", "Heavy Weapons (Artillery)", "Gun Combat (Slug)", "Stealth"],
            specialist_skills: ["Gun Combat (Energy)", "Melee (Blade)", "Leadership", "Medic", "Recon", "Explosives"],
            cash_table: [2_000, 5_000, 5_000, 10_000, 20_000, 30_000, 40_000],
            has_ship_benefit: false,
        },
        Career::Merchant => CareerSpec {
            career,
            name: "Merchant",
            qual_stat: Characteristic::Int,
            qual_target: 4,
            surv_stat: Characteristic::Int,
            surv_target: 5,
            adv_stat: Characteristic::Edu,
            adv_target: 6,
            service_skills: ["Drive (Wheel)", "Vacc Suit", "Broker", "Steward", "Electronics (comms)", "Persuade"],
            specialist_skills: ["Pilot (Spacecraft)", "Astrogation", "Engineer (J-Drive)", "Streetwise", "Admin", "Deception"],
            cash_table: [1_000, 5_000, 10_000, 20_000, 20_000, 40_000, 40_000],
            has_ship_benefit: true,
        },
        Career::Navy => CareerSpec {
            career,
            name: "Navy",
            qual_stat: Characteristic::Int,
            qual_target: 6,
            surv_stat: Characteristic::Int,
            surv_target: 5,
            adv_stat: Characteristic::Edu,
            adv_target: 6,
            service_skills: ["Pilot (Spacecraft)", "Vacc Suit", "Athletics (Dexterity)", "Gunner (Turret)", "Mechanic", "Gun Combat (Energy)"],
            specialist_skills: ["Astrogation", "Electronics (sensors)", "Engineer (Power)", "Engineer (M-Drive)", "Tactics (Naval)", "Leadership"],
            cash_table: [1_000, 5_000, 5_000, 10_000, 20_000, 50_000, 50_000],
            has_ship_benefit: false,
        },
        Career::Noble => CareerSpec {
            career,
            name: "Noble",
            qual_stat: Characteristic::Soc,
            qual_target: 10,
            surv_stat: Characteristic::Soc,
            surv_target: 5,
            adv_stat: Characteristic::Int,
            adv_target: 6,
            service_skills: ["Admin", "Advocate", "Electronics (computers)", "Diplomat", "Investigate", "Persuade"],
            specialist_skills: ["Carouse", "Deception", "Leadership", "Pilot (Small Craft)", "Broker", "Melee (Blade)"],
            cash_table: [10_000, 10_000, 50_000, 50_000, 100_000, 100_000, 200_000],
            has_ship_benefit: true,
        },
        Career::Rogue => CareerSpec {
            career,
            name: "Rogue",
            qual_stat: Characteristic::Dex,
            qual_target: 6,
            surv_stat: Characteristic::Dex,
            surv_target: 6,
            adv_stat: Characteristic::Int,
            adv_target: 6,
            service_skills: ["Deception", "Recon", "Athletics (Dexterity)", "Gun Combat (Slug)", "Stealth", "Streetwise"],
            specialist_skills: ["Electronics (computers)", "Gambler", "Melee (Blade)", "Pilot (Small Craft)", "Astrogation", "Broker"],
            cash_table: [0, 0, 10_000, 10_000, 50_000, 100_000, 100_000],
            has_ship_benefit: true,
        },
        Career::Scholar => CareerSpec {
            career,
            name: "Scholar",
            qual_stat: Characteristic::Int,
            qual_target: 6,
            surv_stat: Characteristic::Edu,
            surv_target: 4,
            adv_stat: Characteristic::Int,
            adv_target: 6,
            service_skills: ["Drive (Wheel)", "Electronics (computers)", "Diplomat", "Medic", "Investigate", "Science (Physics)"],
            specialist_skills: ["Science (Xenology)", "Science (Robotics)", "Science (Cybernetics)", "Astrogation", "Electronics (sensors)", "Language (Vilani)"],
            cash_table: [5_000, 10_000, 20_000, 30_000, 40_000, 60_000, 100_000],
            has_ship_benefit: true,
        },
        Career::Scout => CareerSpec {
            career,
            name: "Scout",
            qual_stat: Characteristic::Int,
            qual_target: 5,
            surv_stat: Characteristic::End,
            surv_target: 7,
            adv_stat: Characteristic::Edu,
            adv_target: 7,
            service_skills: ["Pilot (Spacecraft)", "Survival", "Mechanic", "Astrogation", "Vacc Suit", "Gun Combat (Slug)"],
            specialist_skills: ["Electronics (sensors)", "Electronics (comms)", "Recon", "Jack-of-All-Trades", "Navigation", "Medic"],
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
    pub age: u16,
    pub terms_completed: u16,
    pub upp: Upp,
    pub skills: BTreeMap<String, i32>,
    pub current_career: Option<Career>,
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

/// Start a new MgT2e lifepath character by rolling 6 characteristics (2D6 each) and granting
/// EDU DM + 3 background skills at level 0 (Core 2022 p. 14–16).
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
    let upp = Upp {
        str_val: r2(),
        dex_val: r2(),
        end_val: r2(),
        int_val: r2(),
        edu_val: r2(),
        soc_val: r2(),
        psi_val: None,
    };

    let bg_limit = (characteristic_dm(upp.edu_val) + 3).max(1) as usize;
    let defaults = [
        "Admin",
        "Animals (Handling)",
        "Art (Performer)",
        "Athletics (Endurance)",
        "Carouse",
        "Drive (Wheel)",
        "Electronics (computers)",
        "Mechanic",
        "Medic",
        "Streetwise",
        "Survival",
        "Vacc Suit",
    ];
    let mut skills = BTreeMap::new();
    let chosen: Vec<String> = if background_skills.is_empty() {
        defaults.iter().take(bg_limit).map(|s| (*s).to_string()).collect()
    } else {
        background_skills.into_iter().take(bg_limit).collect()
    };
    for sk in chosen {
        skills.entry(sk).or_insert(0);
    }

    LifepathSession {
        id: Uuid::new_v4(),
        name,
        species,
        age: 18,
        terms_completed: 0,
        upp,
        skills,
        current_career: None,
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
    let spec = career_spec(career);
    let term_num = session.terms_completed + 1;
    let mut skills_gained = Vec::new();

    // 1. Qualification (only if entering a different career)
    let mut qualified = true;
    let entering_new = session.current_career != Some(career);
    let actual_career = if entering_new {
        let d = roll_nd6(rng, 2);
        let roll = i32::from(d[0]) + i32::from(d[1]) + session.upp.dm(spec.qual_stat)
            - (session.terms_completed as i32).min(3);
        if roll >= spec.qual_target {
            session.current_career = Some(career);
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
            session.current_rank = 0;
            Career::Drifter
        }
    } else {
        career
    };

    let active_spec = career_spec(actual_career);

    // 2. Term Skill Roll
    let table = if prefer_specialist {
        &active_spec.specialist_skills
    } else {
        &active_spec.service_skills
    };
    let sk_idx = rng.random_range(0..6);
    skills_gained.push(gain_skill(&mut session.skills, table[sk_idx]));

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

    // 6. Aging Roll (Core 2022 p. 47: at end of 4th term / age 34+, 2D6 - terms_completed)
    let aging_note = if session.age >= 34 {
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

    let entry = LifepathLogEntry {
        term: term_num,
        age: session.age,
        career: active_spec.name.to_string(),
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
        "Generated via MgT2e Lifepath ({} terms, Age {}). Cash Cr{}, Ship Shares {}, Career Ship Benefits {}, Pension Cr{}/yr.",
        session.terms_completed,
        session.age,
        session.cash_cr,
        session.ship_shares,
        session.career_ship_benefits,
        session.pension_cr
    );
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
