use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::universe::Uwp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShipDisposition {
    Friendly,
    Neutral,
    Cautious,
    Hostile,
    Distress,
}

impl ShipDisposition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Friendly => "friendly",
            Self::Neutral => "neutral",
            Self::Cautious => "cautious",
            Self::Hostile => "hostile",
            Self::Distress => "distress",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemVessel {
    pub id: Uuid,
    pub name: String,
    pub ship_class: String,
    pub role: String,
    pub tonnage: i32,
    pub tl: i16,
    pub hull_max: i32,
    pub hull_current: i32,
    pub armor: i32,
    pub thrust: i16,
    pub jump_rating: i16,
    pub sensor_grade: String,
    pub stealth_dm: i16,
    pub transponder_active: bool,
    pub disposition: String,
    pub orbit_location: String,
    pub captain_name: String,
    pub faction: String,
    pub cargo_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemTrafficReport {
    pub system_name: String,
    pub uwp: String,
    pub ship_count: usize,
    pub vessels: Vec<SystemVessel>,
    pub summary: String,
}

struct VesselTemplate {
    prefix: &'static str,
    ship_class: &'static str,
    role: &'static str,
    tonnage: i32,
    tl: i16,
    hull: i32,
    armor: i32,
    thrust: i16,
    jump: i16,
    sensors: &'static str,
    stealth_dm: i16,
    faction: &'static str,
    cargo_hint: &'static str,
}

const MERCHANT_TEMPLATES: &[VesselTemplate] = &[
    VesselTemplate {
        prefix: "ISS",
        ship_class: "Type-A Free Trader",
        role: "merchant",
        tonnage: 200,
        tl: 12,
        hull: 80,
        armor: 2,
        thrust: 1,
        jump: 1,
        sensors: "civilian",
        stealth_dm: 0,
        faction: "Independent Merchant",
        cargo_hint: "Mixed speculative freight & passengers",
    },
    VesselTemplate {
        prefix: "ISS",
        ship_class: "Type-A2 Far Trader",
        role: "merchant",
        tonnage: 200,
        tl: 12,
        hull: 80,
        armor: 2,
        thrust: 1,
        jump: 2,
        sensors: "civilian",
        stealth_dm: 0,
        faction: "Free Merchant Guild",
        cargo_hint: "High-value subsector electronics & mail",
    },
    VesselTemplate {
        prefix: "MV",
        ship_class: "Type-R Subsidized Merchant",
        role: "freighter",
        tonnage: 400,
        tl: 12,
        hull: 160,
        armor: 2,
        thrust: 1,
        jump: 1,
        sensors: "civilian",
        stealth_dm: 0,
        faction: "Tukera Lines",
        cargo_hint: "Bulk industrial goods & charter cargo",
    },
    VesselTemplate {
        prefix: "MV",
        ship_class: "Type-M Subsidized Liner",
        role: "liner",
        tonnage: 600,
        tl: 12,
        hull: 240,
        armor: 2,
        thrust: 1,
        jump: 3,
        sensors: "civilian",
        stealth_dm: 0,
        faction: "Oberlindes Lines",
        cargo_hint: "High & middle passengers, diplomatic pouch",
    },
    VesselTemplate {
        prefix: "SY",
        ship_class: "Type-Y Noble Yacht",
        role: "yacht",
        tonnage: 200,
        tl: 13,
        hull: 80,
        armor: 2,
        thrust: 2,
        jump: 2,
        sensors: "improved",
        stealth_dm: 0,
        faction: "Imperial Nobility",
        cargo_hint: "Luxury consumables & private retinue",
    },
    VesselTemplate {
        prefix: "MV",
        ship_class: "300t Ore Seeker / Belter",
        role: "miner",
        tonnage: 300,
        tl: 11,
        hull: 120,
        armor: 4,
        thrust: 2,
        jump: 1,
        sensors: "improved",
        stealth_dm: 0,
        faction: "Belt Mining Collective",
        cargo_hint: "Refined ore, crystals & radioactives",
    },
];

const PATROL_TEMPLATES: &[VesselTemplate] = &[
    VesselTemplate {
        prefix: "SDB",
        ship_class: "400t System Defense Boat",
        role: "patrol",
        tonnage: 400,
        tl: 13,
        hull: 160,
        armor: 13,
        thrust: 6,
        jump: 0,
        sensors: "military",
        stealth_dm: -2,
        faction: "System Navy / Customs",
        cargo_hint: "Customs boarding marines & torpedo bays",
    },
    VesselTemplate {
        prefix: "INS",
        ship_class: "400t Type-T Patrol Corvette",
        role: "naval_patrol",
        tonnage: 400,
        tl: 13,
        hull: 160,
        armor: 6,
        thrust: 4,
        jump: 3,
        sensors: "military",
        stealth_dm: 0,
        faction: "Imperial Navy",
        cargo_hint: "Anti-piracy patrol, transponder verification",
    },
    VesselTemplate {
        prefix: "IISS",
        ship_class: "100t Type-S Scout/Courier",
        role: "scout",
        tonnage: 100,
        tl: 12,
        hull: 40,
        armor: 4,
        thrust: 2,
        jump: 2,
        sensors: "improved",
        stealth_dm: 0,
        faction: "Imperial Interstellar Scout Service",
        cargo_hint: "X-Boat mail relay & survey probes",
    },
];

const FRINGE_TEMPLATES: &[VesselTemplate] = &[
    VesselTemplate {
        prefix: "VCS",
        ship_class: "400t Vargr Corsair Raider",
        role: "corsair",
        tonnage: 400,
        tl: 12,
        hull: 160,
        armor: 4,
        thrust: 4,
        jump: 2,
        sensors: "military",
        stealth_dm: -2,
        faction: "Vargr Corsair Band",
        cargo_hint: "Illicit salvage & hidden contraband holds",
    },
    VesselTemplate {
        prefix: "SS",
        ship_class: "100t Stealth Smuggler",
        role: "smuggler",
        tonnage: 100,
        tl: 13,
        hull: 40,
        armor: 2,
        thrust: 4,
        jump: 2,
        sensors: "improved",
        stealth_dm: -4,
        faction: "Shadow Syndicate",
        cargo_hint: "Unmanifested restricted cargo (transponder dark)",
    },
    VesselTemplate {
        prefix: "ISS",
        ship_class: "200t Damaged Salvage Hauler",
        role: "distress",
        tonnage: 200,
        tl: 11,
        hull: 80,
        armor: 2,
        thrust: 1,
        jump: 1,
        sensors: "basic",
        stealth_dm: 0,
        faction: "Independent Salvager",
        cargo_hint: "Broadcasting low-power engineering distress beacon",
    },
];

const SHIP_NAMES: &[&str] = &[
    "Meridian Star",
    "Vigilant",
    "Golden Hind",
    "March Harrier",
    "Regina Dawn",
    "Silent Vector",
    "Kestrel's Wing",
    "Silver Sovereign",
    "Iron Horizon",
    "Suleiman's Pride",
    "Wayfarer",
    "Crimson Comet",
    "Valkyrie",
    "Starlight Venture",
    "Void Runner",
    "Gilded Astrolabe",
    "Aurora Drift",
    "Osprey",
    "Sable Fang",
    "Nomad's Fortune",
    "Highport Express",
    "Aegis",
    "Rift Dancer",
    "Calypso",
];

const CAPTAINS: &[&str] = &[
    "Capt. Elena Rostova",
    "Capt. Marcus Vance",
    "Cmdr. Hault-Oberlindes",
    "Capt. Soren Lindqvist",
    "Capt. Talia Reyes",
    "Skipper Bram Okafor",
    "Capt. Yuki Sato",
    "Cmdr. Vesper Thorne",
    "Corsair Krrsh-Varr",
    "Capt. Nadia Al-Mansoor",
    "Skipper Joren Barrow",
    "Capt. Linnea Solis",
];

const ORBITS: &[&str] = &[
    "Highport Docking Ring",
    "Mainworld High Orbit",
    "100D Jump Limit (Inbound)",
    "100D Jump Limit (Outbound)",
    "Gas Giant Skimming Vector",
    "Outer System Asteroid Belt",
];

/// Generate authentic MgT2e in-system starship traffic scaled by Starport Class, Population,
/// Naval/Scout bases, and Travel Zone (Green / Amber / Red).
pub fn generate_system_traffic<R: Rng + ?Sized>(
    rng: &mut R,
    system_name: &str,
    uwp: &Uwp,
    bases: &[String],
    travel_zone: &str,
) -> SystemTrafficReport {
    let (min_ships, max_ships) = match uwp.starport {
        'A' => (7usize, 14usize),
        'B' => (5, 10),
        'C' => (3, 6),
        'D' => (1, 4),
        'E' => (0, 2),
        _ => (0, 2),
    };

    let pop_bonus: usize = if uwp.population >= 9 {
        2
    } else if uwp.population >= 7 {
        1
    } else {
        0
    };
    let is_red = travel_zone.eq_ignore_ascii_case("Red");
    let is_amber = travel_zone.eq_ignore_ascii_case("Amber");
    let has_naval = bases.iter().any(|b| b.eq_ignore_ascii_case("N"));
    let has_scout = bases.iter().any(|b| b.eq_ignore_ascii_case("S"));

    let mut target_count = if is_red {
        rng.random_range(1..=3)
    } else {
        rng.random_range(min_ships..=max_ships) + pop_bonus
    };
    if has_naval && target_count < 2 {
        target_count = 2;
    }

    let mut vessels = Vec::with_capacity(target_count);
    let mut used_names = std::collections::HashSet::new();

    for i in 0..target_count {
        let (tpl, disp, transponder, hull_pct) = if is_red && i == 0 {
            (&PATROL_TEMPLATES[1], ShipDisposition::Hostile, true, 100)
        } else if has_naval && i == 0 {
            (&PATROL_TEMPLATES[1], ShipDisposition::Neutral, true, 100)
        } else if has_scout && i == 1 {
            (&PATROL_TEMPLATES[2], ShipDisposition::Friendly, true, 100)
        } else if matches!(uwp.starport, 'A' | 'B') && i == 0 {
            (&PATROL_TEMPLATES[0], ShipDisposition::Neutral, true, 100)
        } else {
            let roll = rng.random_range(0..100);
            if roll < (if is_amber || is_red { 40 } else { 18 }) {
                let ft = &FRINGE_TEMPLATES[rng.random_range(0..FRINGE_TEMPLATES.len())];
                let d = if ft.role == "distress" {
                    ShipDisposition::Distress
                } else if ft.role == "corsair" {
                    ShipDisposition::Hostile
                } else {
                    ShipDisposition::Cautious
                };
                let xpdr = ft.role == "distress";
                let hp = if ft.role == "distress" { 45 } else { 100 };
                (ft, d, xpdr, hp)
            } else if roll < 32 {
                let pt = &PATROL_TEMPLATES[rng.random_range(0..PATROL_TEMPLATES.len())];
                (pt, ShipDisposition::Neutral, true, 100)
            } else {
                let mt = &MERCHANT_TEMPLATES[rng.random_range(0..MERCHANT_TEMPLATES.len())];
                let d = if rng.random_range(0..3) == 0 {
                    ShipDisposition::Friendly
                } else {
                    ShipDisposition::Neutral
                };
                (mt, d, true, 100)
            }
        };

        let mut base_name = SHIP_NAMES[rng.random_range(0..SHIP_NAMES.len())].to_string();
        if used_names.contains(&base_name) {
            base_name = format!("{base_name} {}", i + 1);
        }
        used_names.insert(base_name.clone());

        let orbit = if !transponder {
            "Outer System / Sensor Shadow"
        } else if tpl.role == "patrol" || tpl.role == "naval_patrol" {
            "Mainworld High Orbit"
        } else if tpl.role == "distress" {
            "100D Jump Limit (Drifting)"
        } else {
            ORBITS[rng.random_range(0..ORBITS.len())]
        };

        let hull_cur = ((tpl.hull * hull_pct) / 100).max(1);
        vessels.push(SystemVessel {
            id: Uuid::new_v4(),
            name: format!("{} {}", tpl.prefix, base_name),
            ship_class: tpl.ship_class.to_string(),
            role: tpl.role.to_string(),
            tonnage: tpl.tonnage,
            tl: tpl.tl.min(i16::from(uwp.tech_level.max(10)) + 2),
            hull_max: tpl.hull,
            hull_current: hull_cur,
            armor: tpl.armor,
            thrust: tpl.thrust,
            jump_rating: tpl.jump,
            sensor_grade: tpl.sensors.to_string(),
            stealth_dm: tpl.stealth_dm,
            transponder_active: transponder,
            disposition: disp.as_str().to_string(),
            orbit_location: orbit.to_string(),
            captain_name: CAPTAINS[rng.random_range(0..CAPTAINS.len())].to_string(),
            faction: tpl.faction.to_string(),
            cargo_hint: tpl.cargo_hint.to_string(),
        });
    }

    let ship_count = vessels.len();
    let summary = if ship_count == 0 {
        format!("Sensors sweep {system_name} [{}]: space is empty (0 vessels detected).", uwp.raw)
    } else {
        let dark_count = vessels.iter().filter(|v| !v.transponder_active).count();
        let hostile_count = vessels.iter().filter(|v| v.disposition == "hostile").count();
        let distress_count = vessels.iter().filter(|v| v.disposition == "distress").count();
        let mut s = format!(
            "Sensors sweep {system_name} [{}]: {ship_count} ship(s) detected in-system",
            uwp.raw
        );
        let mut notes = Vec::new();
        if dark_count > 0 {
            notes.push(format!("{dark_count} running transponder-dark"));
        }
        if hostile_count > 0 {
            notes.push(format!("{hostile_count} potentially hostile"));
        }
        if distress_count > 0 {
            notes.push(format!("{distress_count} broadcasting distress"));
        }
        if !notes.is_empty() {
            s.push_str(&format!(" ({})", notes.join(", ")));
        }
        s.push('.');
        s
    };

    SystemTrafficReport {
        system_name: system_name.to_string(),
        uwp: uwp.raw.clone(),
        ship_count,
        vessels,
        summary,
    }
}
