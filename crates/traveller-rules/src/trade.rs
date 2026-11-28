use std::collections::BTreeMap;

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::dice::roll_nd6;
use crate::universe::Uwp;

/// Core 2022 p. 241: Passage & Freight base rates per parsec jump (1..=6 parsecs).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ParsecRates {
    pub high_passage: i64,
    pub middle_passage: i64,
    pub basic_passage: i64,
    pub low_passage: i64,
    pub freight_per_ton: i64,
}

pub fn parsec_rates(parsecs: u8) -> ParsecRates {
    match parsecs.clamp(1, 6) {
        1 => ParsecRates { high_passage: 9_000, middle_passage: 6_500, basic_passage: 2_000, low_passage: 700, freight_per_ton: 1_000 },
        2 => ParsecRates { high_passage: 14_000, middle_passage: 10_000, basic_passage: 3_000, low_passage: 1_300, freight_per_ton: 1_600 },
        3 => ParsecRates { high_passage: 21_000, middle_passage: 14_000, basic_passage: 5_000, low_passage: 2_200, freight_per_ton: 2_600 },
        4 => ParsecRates { high_passage: 30_000, middle_passage: 19_000, basic_passage: 7_000, low_passage: 3_300, freight_per_ton: 4_000 },
        5 => ParsecRates { high_passage: 40_000, middle_passage: 25_000, basic_passage: 10_000, low_passage: 4_500, freight_per_ton: 5_500 },
        _ => ParsecRates { high_passage: 50_000, middle_passage: 30_000, basic_passage: 15_000, low_passage: 6_000, freight_per_ton: 7_000 },
    }
}

/// Core 2022 p. 243: Modified Price table (3D6 + Broker + Trade Code DMs -> Purchase % & Sale %).
pub fn modified_price_percent(roll: i32) -> (u32, u32) {
    match roll {
        i32::MIN..=-3 => (300, 10),
        -2 => (250, 20),
        -1 => (200, 30),
        0 => (175, 40),
        1 => (150, 45),
        2 => (135, 50),
        3 => (125, 55),
        4 => (120, 60),
        5 => (115, 65),
        6 => (110, 70),
        7 => (105, 75),
        8 => (100, 80),
        9 => (95, 85),
        10 => (90, 90),
        11 => (85, 100),
        12 => (80, 105),
        13 => (75, 110),
        14 => (70, 115),
        15 => (65, 120),
        16 => (60, 125),
        17 => (55, 130),
        18 => (50, 140),
        19 => (45, 150),
        20 => (40, 160),
        21 => (35, 175),
        22 => (30, 200),
        23 => (25, 250),
        24 => (20, 300),
        _ => (15, 400),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TradeGoodSpec {
    pub d66: u8,
    pub name: &'static str,
    pub availability: &'static [&'static str], // empty = "All"
    pub tons_dice: u8,
    pub tons_mult: u32,
    pub base_price_cr: i64,
    pub purchase_dms: &'static [(&'static str, i32)],
    pub sale_dms: &'static [(&'static str, i32)],
    pub illegal: bool,
}

/// All canonical MgT2e Core Rulebook Update 2022 pp. 244–245 Trade Goods (D66 11..=65).
pub fn all_trade_goods() -> &'static [TradeGoodSpec] {
    &[
        TradeGoodSpec { d66: 11, name: "Common Electronics", availability: &[], tons_dice: 2, tons_mult: 10, base_price_cr: 20_000, purchase_dms: &[("In", 2), ("Ht", 3), ("Ri", 1)], sale_dms: &[("Ni", 2), ("Lt", 1), ("Po", 1)], illegal: false },
        TradeGoodSpec { d66: 12, name: "Common Industrial Goods", availability: &[], tons_dice: 2, tons_mult: 10, base_price_cr: 10_000, purchase_dms: &[("Na", 2), ("In", 5)], sale_dms: &[("Ni", 3), ("Ag", 2)], illegal: false },
        TradeGoodSpec { d66: 13, name: "Common Manufactured Goods", availability: &[], tons_dice: 2, tons_mult: 10, base_price_cr: 20_000, purchase_dms: &[("Na", 2), ("In", 5)], sale_dms: &[("Ni", 3), ("Hi", 2)], illegal: false },
        TradeGoodSpec { d66: 14, name: "Common Raw Materials", availability: &[], tons_dice: 2, tons_mult: 20, base_price_cr: 5_000, purchase_dms: &[("Ag", 3), ("Ga", 2)], sale_dms: &[("In", 2), ("Po", 2)], illegal: false },
        TradeGoodSpec { d66: 15, name: "Common Consumables", availability: &[], tons_dice: 2, tons_mult: 20, base_price_cr: 500, purchase_dms: &[("Ag", 3), ("Wa", 2), ("Ga", 1), ("As", -4)], sale_dms: &[("As", 1), ("Fl", 1), ("Ic", 1), ("Hi", 1)], illegal: false },
        TradeGoodSpec { d66: 16, name: "Common Ore", availability: &[], tons_dice: 2, tons_mult: 20, base_price_cr: 1_000, purchase_dms: &[("As", 4)], sale_dms: &[("In", 3), ("Ni", 1)], illegal: false },
        TradeGoodSpec { d66: 21, name: "Advanced Electronics", availability: &["In", "Ht"], tons_dice: 1, tons_mult: 5, base_price_cr: 100_000, purchase_dms: &[("In", 2), ("Ht", 3)], sale_dms: &[("Ni", 1), ("Ri", 2), ("As", 3)], illegal: false },
        TradeGoodSpec { d66: 22, name: "Advanced Machine Parts", availability: &["In", "Ht"], tons_dice: 1, tons_mult: 5, base_price_cr: 75_000, purchase_dms: &[("In", 2), ("Ht", 1)], sale_dms: &[("As", 2), ("Ni", 1)], illegal: false },
        TradeGoodSpec { d66: 23, name: "Advanced Manufactured Goods", availability: &["In", "Ht"], tons_dice: 1, tons_mult: 5, base_price_cr: 100_000, purchase_dms: &[("In", 1)], sale_dms: &[("Hi", 1), ("Ri", 2)], illegal: false },
        TradeGoodSpec { d66: 24, name: "Advanced Weapons", availability: &["In", "Ht"], tons_dice: 1, tons_mult: 5, base_price_cr: 150_000, purchase_dms: &[("Ht", 2)], sale_dms: &[("Po", 1), ("Amber", 2), ("Red", 4)], illegal: false },
        TradeGoodSpec { d66: 25, name: "Advanced Vehicles", availability: &["In", "Ht"], tons_dice: 1, tons_mult: 5, base_price_cr: 180_000, purchase_dms: &[("Ht", 2)], sale_dms: &[("As", 2), ("Ri", 2)], illegal: false },
        TradeGoodSpec { d66: 26, name: "Biochemicals", availability: &["Ag", "Wa"], tons_dice: 1, tons_mult: 5, base_price_cr: 50_000, purchase_dms: &[("Ag", 1), ("Wa", 2)], sale_dms: &[("In", 2)], illegal: false },
        TradeGoodSpec { d66: 31, name: "Crystals & Gems", availability: &["As", "De", "Ic"], tons_dice: 1, tons_mult: 5, base_price_cr: 20_000, purchase_dms: &[("As", 2), ("De", 1), ("Ic", 1)], sale_dms: &[("In", 3), ("Ri", 2)], illegal: false },
        TradeGoodSpec { d66: 32, name: "Cybernetics", availability: &["Ht"], tons_dice: 1, tons_mult: 1, base_price_cr: 250_000, purchase_dms: &[("Ht", 1)], sale_dms: &[("As", 1), ("Ic", 1), ("Ri", 2)], illegal: false },
        TradeGoodSpec { d66: 33, name: "Live Animals", availability: &["Ag", "Ga"], tons_dice: 1, tons_mult: 10, base_price_cr: 10_000, purchase_dms: &[("Ag", 2)], sale_dms: &[("Lo", 3)], illegal: false },
        TradeGoodSpec { d66: 34, name: "Luxury Consumables", availability: &["Ag", "Ga", "Wa"], tons_dice: 1, tons_mult: 10, base_price_cr: 20_000, purchase_dms: &[("Ag", 2), ("Wa", 1)], sale_dms: &[("Ri", 2), ("Hi", 2)], illegal: false },
        TradeGoodSpec { d66: 35, name: "Luxury Goods", availability: &["Hi"], tons_dice: 1, tons_mult: 1, base_price_cr: 200_000, purchase_dms: &[("Hi", 1)], sale_dms: &[("Ri", 4)], illegal: false },
        TradeGoodSpec { d66: 36, name: "Medical Supplies", availability: &["Ht", "Hi"], tons_dice: 1, tons_mult: 5, base_price_cr: 50_000, purchase_dms: &[("Ht", 2)], sale_dms: &[("In", 2), ("Po", 1), ("Ri", 1)], illegal: false },
        TradeGoodSpec { d66: 41, name: "Petrochemicals", availability: &["De", "Fl", "Ic", "Wa"], tons_dice: 1, tons_mult: 10, base_price_cr: 10_000, purchase_dms: &[("De", 2)], sale_dms: &[("In", 2), ("Ag", 1), ("Lt", 2)], illegal: false },
        TradeGoodSpec { d66: 42, name: "Pharmaceuticals", availability: &["As", "De", "Hi", "Wa"], tons_dice: 1, tons_mult: 1, base_price_cr: 100_000, purchase_dms: &[("As", 2), ("Hi", 1)], sale_dms: &[("Ri", 2), ("Lt", 1)], illegal: false },
        TradeGoodSpec { d66: 43, name: "Polymers", availability: &["In"], tons_dice: 1, tons_mult: 10, base_price_cr: 7_000, purchase_dms: &[("In", 1)], sale_dms: &[("Ri", 2), ("Ni", 1)], illegal: false },
        TradeGoodSpec { d66: 44, name: "Precious Metals", availability: &["As", "De", "Ic", "Fl"], tons_dice: 1, tons_mult: 1, base_price_cr: 50_000, purchase_dms: &[("As", 3), ("De", 1), ("Ic", 2)], sale_dms: &[("Ri", 3), ("In", 2), ("Ht", 1)], illegal: false },
        TradeGoodSpec { d66: 45, name: "Radioactives", availability: &["As", "De", "Lo"], tons_dice: 1, tons_mult: 1, base_price_cr: 1_000_000, purchase_dms: &[("As", 2), ("Lo", 2)], sale_dms: &[("In", 3), ("Ht", 1), ("Ni", -2), ("Ag", -3)], illegal: false },
        TradeGoodSpec { d66: 46, name: "Robots", availability: &["In"], tons_dice: 1, tons_mult: 5, base_price_cr: 400_000, purchase_dms: &[("In", 1)], sale_dms: &[("Ag", 2), ("Ht", 1)], illegal: false },
        TradeGoodSpec { d66: 51, name: "Spices", availability: &["Ga", "De", "Wa"], tons_dice: 1, tons_mult: 10, base_price_cr: 6_000, purchase_dms: &[("De", 2)], sale_dms: &[("Hi", 2), ("Ri", 3), ("Po", 3)], illegal: false },
        TradeGoodSpec { d66: 52, name: "Textiles", availability: &["Ag", "Ni"], tons_dice: 1, tons_mult: 20, base_price_cr: 3_000, purchase_dms: &[("Ag", 7)], sale_dms: &[("Hi", 3), ("Na", 2)], illegal: false },
        TradeGoodSpec { d66: 53, name: "Uncommon Ore", availability: &["As", "Ic"], tons_dice: 1, tons_mult: 20, base_price_cr: 5_000, purchase_dms: &[("As", 4)], sale_dms: &[("In", 3), ("Ni", 1)], illegal: false },
        TradeGoodSpec { d66: 54, name: "Uncommon Raw Materials", availability: &["Ag", "De", "Wa"], tons_dice: 1, tons_mult: 10, base_price_cr: 20_000, purchase_dms: &[("Ag", 2), ("Wa", 1)], sale_dms: &[("In", 2), ("Ht", 1)], illegal: false },
        TradeGoodSpec { d66: 55, name: "Wood", availability: &["Ag", "Ga"], tons_dice: 1, tons_mult: 20, base_price_cr: 1_000, purchase_dms: &[("Ag", 6)], sale_dms: &[("Ri", 2), ("In", 1)], illegal: false },
        TradeGoodSpec { d66: 56, name: "Vehicles", availability: &["In", "Ht"], tons_dice: 1, tons_mult: 10, base_price_cr: 15_000, purchase_dms: &[("In", 2), ("Ht", 1)], sale_dms: &[("Ni", 2), ("Hi", 1)], illegal: false },
        TradeGoodSpec { d66: 61, name: "Illegal Biochemicals", availability: &["Ag", "Wa"], tons_dice: 1, tons_mult: 5, base_price_cr: 50_000, purchase_dms: &[("Wa", 2)], sale_dms: &[("In", 6)], illegal: true },
        TradeGoodSpec { d66: 62, name: "Illegal Cybernetics", availability: &["Ht"], tons_dice: 1, tons_mult: 1, base_price_cr: 250_000, purchase_dms: &[("Ht", 1)], sale_dms: &[("As", 4), ("Ic", 4), ("Ri", 8), ("Amber", 6), ("Red", 6)], illegal: true },
        TradeGoodSpec { d66: 63, name: "Illegal Drugs", availability: &["As", "De", "Hi", "Wa"], tons_dice: 1, tons_mult: 1, base_price_cr: 100_000, purchase_dms: &[("As", 1), ("De", 1), ("Ga", 1), ("Wa", 1)], sale_dms: &[("Ri", 6), ("Hi", 6)], illegal: true },
        TradeGoodSpec { d66: 64, name: "Illegal Luxuries", availability: &["Ag", "Ga", "Wa"], tons_dice: 1, tons_mult: 1, base_price_cr: 50_000, purchase_dms: &[("Ag", 2), ("Wa", 1)], sale_dms: &[("Ri", 6), ("Hi", 4)], illegal: true },
        TradeGoodSpec { d66: 65, name: "Illegal Weapons", availability: &["In", "Ht"], tons_dice: 1, tons_mult: 5, base_price_cr: 150_000, purchase_dms: &[("Ht", 2)], sale_dms: &[("Po", 6), ("Amber", 8), ("Red", 10)], illegal: true },
    ]
}

fn best_dm(table: &[(&str, i32)], codes: &[String], travel_zone: &str) -> i32 {
    let mut best = 0;
    for &(code, dm) in table {
        let matched = codes.iter().any(|c| c.eq_ignore_ascii_case(code))
            || travel_zone.eq_ignore_ascii_case(code);
        if matched && (best == 0 || dm > best) {
            best = dm;
        }
    }
    best
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreightLot {
    pub lot_type: String, // Major, Minor, Incidental
    pub tons: u32,
    pub payout_cr: i64,
    pub destination: String,
    pub parsecs: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassengerAvailability {
    pub high: u32,
    pub middle: u32,
    pub basic: u32,
    pub low: u32,
    pub rates: ParsecRates,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeculativeOffer {
    pub d66: u8,
    pub name: String,
    pub tons_available: u32,
    pub base_price_cr: i64,
    pub price_percent: u32,
    pub unit_price_cr: i64,
    pub illegal: bool,
    pub purchase_dm: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldMarket {
    pub system_name: String,
    pub uwp: String,
    pub trade_codes: Vec<String>,
    pub freight: Vec<FreightLot>,
    pub passengers: PassengerAvailability,
    pub mail_available: bool,
    pub mail_payout_cr: i64,
    pub speculative_goods: Vec<SpeculativeOffer>,
}

fn traffic_dice_for_roll(total: i32) -> u8 {
    match total {
        i32::MIN..=1 => 0,
        2..=3 => 1,
        4..=6 => 2,
        7..=10 => 3,
        11..=13 => 4,
        14..=15 => 5,
        16 => 6,
        17 => 7,
        18 => 8,
        19 => 9,
        _ => 10,
    }
}

fn world_traffic_dm(uwp: &Uwp) -> i32 {
    let pop_dm = match uwp.population {
        0..=1 => -4,
        6..=7 => 2,
        8..=20 => 4,
        _ => 0,
    };
    let port_dm = match uwp.starport {
        'A' => 2,
        'B' => 1,
        'E' => -1,
        'X' => -3,
        _ => 0,
    };
    let tl_dm = if uwp.tech_level <= 6 {
        -1
    } else if uwp.tech_level >= 9 {
        2
    } else {
        0
    };
    pop_dm + port_dm + tl_dm
}

/// Generate a complete Core 2022 World Market (Freight, Passengers, Mail, Speculative Trade Lots).
#[allow(clippy::too_many_arguments)]
pub fn generate_world_market<R: Rng + ?Sized>(
    rng: &mut R,
    origin_name: &str,
    origin: &Uwp,
    origin_zone: &str,
    dest_name: &str,
    dest: &Uwp,
    parsecs: u8,
    broker_skill: i32,
    steward_skill: i32,
    include_illegal: bool,
) -> WorldMarket {
    let rates = parsec_rates(parsecs);
    let base_dm = world_traffic_dm(origin) + world_traffic_dm(dest) - i32::from(parsecs.saturating_sub(1));

    // 1. Freight lots (Major 1D*10t, Minor 1D*5t, Incidental 1Dt)
    let mut freight = Vec::new();
    for (label, mod_dm, mult) in [("Major", -4, 10u32), ("Minor", 0, 5), ("Incidental", 2, 1)] {
        let d = roll_nd6(rng, 2);
        let roll = i32::from(d[0]) + i32::from(d[1]) + base_dm + mod_dm;
        let lots_dice = traffic_dice_for_roll(roll);
        let lot_count: u32 = if lots_dice == 0 {
            0
        } else {
            roll_nd6(rng, usize::from(lots_dice))
                .into_iter()
                .map(u32::from)
                .sum::<u32>()
                .min(6)
        };
        for _ in 0..lot_count {
            let tons = u32::from(roll_nd6(rng, 1)[0]) * mult;
            freight.push(FreightLot {
                lot_type: label.into(),
                tons,
                payout_cr: i64::from(tons) * rates.freight_per_ton,
                destination: dest_name.into(),
                parsecs,
            });
        }
    }

    // 2. Passengers (High, Middle, Basic, Low)
    let mut roll_pax = |extra_dm: i32| -> u32 {
        let d = roll_nd6(rng, 2);
        let total = i32::from(d[0]) + i32::from(d[1]) + base_dm + steward_skill + extra_dm;
        let dice = traffic_dice_for_roll(total);
        if dice == 0 {
            0
        } else {
            roll_nd6(rng, usize::from(dice))
                .into_iter()
                .map(u32::from)
                .sum()
        }
    };
    let passengers = PassengerAvailability {
        high: roll_pax(-4),
        middle: roll_pax(0),
        basic: roll_pax(0),
        low: roll_pax(1),
        rates,
    };

    // 3. Mail contract (Core 2022 p. 242: 2D6 + freight traffic DM >= 12 -> 5t, Cr25,000)
    let mail_d = roll_nd6(rng, 2);
    let mail_available = i32::from(mail_d[0]) + i32::from(mail_d[1]) + base_dm >= 12;

    // 4. Speculative Trade Goods (all Common goods + goods matching Origin Trade Codes)
    let mut speculative_goods = Vec::new();
    let supplier_broker = 2; // Standard supplier Broker-2 (Core 2022 p. 243)
    for g in all_trade_goods() {
        if g.illegal && !include_illegal {
            continue;
        }
        let available = g.availability.is_empty()
            || g.availability
                .iter()
                .any(|code| origin.trade_codes.iter().any(|c| c.eq_ignore_ascii_case(code)));
        if !available {
            continue;
        }
        let tons: u32 = roll_nd6(rng, usize::from(g.tons_dice))
            .into_iter()
            .map(u32::from)
            .sum::<u32>()
            * g.tons_mult;

        let p_dm = best_dm(g.purchase_dms, &origin.trade_codes, origin_zone);
        let s_dm = best_dm(g.sale_dms, &origin.trade_codes, origin_zone);
        let price_dice = roll_nd6(rng, 3);
        let price_roll = i32::from(price_dice[0])
            + i32::from(price_dice[1])
            + i32::from(price_dice[2])
            + broker_skill
            + p_dm
            - s_dm
            - supplier_broker;
        let (purchase_pct, _) = modified_price_percent(price_roll);
        let unit_price_cr = (g.base_price_cr * i64::from(purchase_pct)) / 100;

        speculative_goods.push(SpeculativeOffer {
            d66: g.d66,
            name: g.name.to_string(),
            tons_available: tons,
            base_price_cr: g.base_price_cr,
            price_percent: purchase_pct,
            unit_price_cr,
            illegal: g.illegal,
            purchase_dm: p_dm,
        });
    }

    WorldMarket {
        system_name: origin_name.to_string(),
        uwp: origin.raw.clone(),
        trade_codes: origin.trade_codes.clone(),
        freight,
        passengers,
        mail_available,
        mail_payout_cr: if mail_available { 25_000 } else { 0 },
        speculative_goods,
    }
}

/// Core 2022 p. 241–242: Searching for a Local Starport Broker (takes 1D days, charges 5% per skill level).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalBrokerOffer {
    pub found: bool,
    pub broker_name: String,
    pub broker_skill: i32,
    pub commission_pct: u32,
    pub days_spent: u32,
    pub starport: char,
    pub note: String,
}

pub fn search_local_broker<R: Rng + ?Sized>(
    rng: &mut R,
    starport: char,
    population: u8,
    black_market: bool,
) -> LocalBrokerOffer {
    let sp = starport.to_ascii_uppercase();
    let days_spent = u32::from(roll_nd6(rng, 1)[0]);
    if sp == 'X' || population == 0 {
        return LocalBrokerOffer {
            found: false,
            broker_name: String::new(),
            broker_skill: 0,
            commission_pct: 0,
            days_spent,
            starport: sp,
            note: format!(
                "Searched for {days_spent} day(s), but no commercial brokers operate on this world (Starport {sp})."
            ),
        };
    }

    let max_skill = match sp {
        'A' => 4,
        'B' => 3,
        'C' => 2,
        'D' => 2,
        _ => 1,
    };
    let port_dm = match sp {
        'A' => 2,
        'B' => 1,
        'C' => 0,
        'D' => -1,
        _ => -2,
    };
    let pop_dm = if population >= 8 {
        2
    } else if population >= 5 {
        1
    } else if population <= 2 {
        -2
    } else {
        0
    };

    let d = roll_nd6(rng, 2);
    let roll = i32::from(d[0]) + i32::from(d[1]) + port_dm + pop_dm;
    if roll < 6 {
        return LocalBrokerOffer {
            found: false,
            broker_name: String::new(),
            broker_skill: 0,
            commission_pct: 0,
            days_spent,
            starport: sp,
            note: format!(
                "Spent {days_spent} day(s) networking at Starport {sp} (rolled {roll}/6+) — no reputable local broker available right now."
            ),
        };
    }

    let raw_skill = match roll {
        6..=7 => 1,
        8..=9 => 2,
        10..=11 => 3,
        _ => 4,
    };
    let broker_skill = raw_skill.min(max_skill).max(1);
    let commission_pct = (broker_skill as u32) * 5;
    let first_names = [
        "Varek", "Sera", "Kellan", "Nadia", "Tovin", "Lira", "Marek", "Dax", "Olen", "Yara",
    ];
    let last_names = [
        "Harrow", "Voss", "Kestrel", "Solin", "Marlowe", "Renn", "Corvus", "Talek", "Vane", "Zor",
    ];
    let fn_i = rng.random_range(0..first_names.len());
    let ln_i = rng.random_range(0..last_names.len());
    let broker_name = format!(
        "{} {}{}",
        first_names[fn_i],
        last_names[ln_i],
        if black_market { " (Fixer)" } else { " (Licensed Factor)" }
    );

    LocalBrokerOffer {
        found: true,
        broker_name: broker_name.clone(),
        broker_skill,
        commission_pct,
        days_spent,
        starport: sp,
        note: format!(
            "Located {broker_name} after {days_spent} day(s) at Starport {sp}: Broker-{broker_skill} ({commission_pct}% commission)."
        ),
    }
}

/// Core 2022 p. 243: Roll speculative cargo sale price on destination world.
pub fn roll_speculative_sale<R: Rng + ?Sized>(
    rng: &mut R,
    d66: u8,
    dest: &Uwp,
    dest_zone: &str,
    broker_skill: i32,
    buyer_broker: i32,
) -> Option<(u32, i64, i32)> {
    let g = all_trade_goods().iter().find(|x| x.d66 == d66)?;
    let s_dm = best_dm(g.sale_dms, &dest.trade_codes, dest_zone);
    let p_dm = best_dm(g.purchase_dms, &dest.trade_codes, dest_zone);
    let d = roll_nd6(rng, 3);
    let total = i32::from(d[0]) + i32::from(d[1]) + i32::from(d[2]) + broker_skill + s_dm - p_dm
        - buyer_broker;
    let (_, sale_pct) = modified_price_percent(total);
    let unit_price = (g.base_price_cr * i64::from(sale_pct)) / 100;
    Some((sale_pct, unit_price, total))
}

/// Summarize ship cargo hold tonnage & value.
pub fn summarize_cargo_tons(cargo_items: &[BTreeMap<String, serde_json::Value>]) -> f64 {
    cargo_items
        .iter()
        .filter_map(|c| c.get("tons").and_then(|v| v.as_f64()))
        .sum()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetBrokerSummary {
    pub max_broker_skill: i32,
    pub broker_source: String,
    pub max_steward_skill: i32,
    pub steward_source: String,
    pub contributors: Vec<(String, i32)>,
}

fn effective_skill_with_augments(c: &crate::character::Character, skill_name: &str) -> (i32, String) {
    let raw = c.raw_skill_level(skill_name);
    let mut best_expert: Option<u8> = None;
    for aug in &c.augments {
        let loaded = match aug {
            crate::character::Augment::NeuralJack { loaded_expert, .. }
            | crate::character::Augment::WaferJack { loaded_expert, .. } => loaded_expert,
            _ => continue,
        };
        for sw in loaded {
            if sw.active && sw.skill.eq_ignore_ascii_case(skill_name) {
                best_expert = Some(best_expert.map_or(sw.rating, |b| b.max(sw.rating)));
            }
        }
    }
    let has_skill_aug = raw.is_some()
        && c.augments.iter().any(|a| {
            matches!(a, crate::character::Augment::SkillAugmentation { skill: s } if s.eq_ignore_ascii_case(skill_name))
        });

    let mut eff = match (raw, best_expert) {
        (Some(lvl), Some(_)) => lvl + 1, // Core p. 112: Expert grants DM+1 if skill possessed
        (Some(lvl), None) => lvl,
        (None, Some(r)) => i32::from(r).saturating_sub(1), // Unskilled uses Bandwidth - 1
        (None, None) => 0,
    };
    if has_skill_aug {
        eff += 1;
    }

    let suffix = match (best_expert, has_skill_aug) {
        (Some(r), true) => format!(" (Expert/{r} + Skill Aug)"),
        (Some(r), false) => format!(" (Expert/{r})"),
        (None, true) => " (Skill Aug)".into(),
        (None, false) => String::new(),
    };
    let species_tag = if c.species.to_lowercase().contains("robot") {
        " [Robot]"
    } else {
        ""
    };
    (eff, format!("{}{species_tag}{suffix}", c.name))
}

fn extract_item_skill(item: &serde_json::Value, skill_key: &str) -> Option<(String, i32)> {
    let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("Item");
    let stats = item.get("stats").unwrap_or(item);
    let lower_key = skill_key.to_lowercase();

    // 1. Direct numeric stat e.g. stats.broker = 2 or stats.broker_skill = 2
    for field in [lower_key.as_str(), "broker_skill", "steward_skill"] {
        if field.starts_with(&lower_key)
            && let Some(val) = stats.get(field).and_then(|v| v.as_i64())
        {
            return Some((format!("{name} [Robot/Gear]"), val as i32));
        }
    }

    // 2. Nested skills map e.g. stats.skills = {"Broker": 3}
    if let Some(obj) = stats.get("skills").and_then(|v| v.as_object()) {
        for (k, v) in obj {
            if k.to_lowercase() == lower_key
                && let Some(lvl) = v.as_i64()
            {
                return Some((format!("{name} [Robot/Gear]"), lvl as i32));
            }
        }
    }

    // 3. Parse text field e.g. stats.effect or stats.software containing "Broker-2" or "Broker 2"
    for field in ["effect", "skills", "software", "traits", "notes"] {
        if let Some(s) = stats.get(field).and_then(|v| v.as_str()) {
            let sl = s.to_lowercase();
            if let Some(pos) = sl.find(&lower_key) {
                let tail = &sl[pos + lower_key.len()..];
                let num_str: String = tail
                    .chars()
                    .skip_while(|c| !c.is_ascii_digit())
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                if let Ok(lvl) = num_str.parse::<i32>() {
                    return Some((format!("{name} [Robot/Gear]"), lvl));
                }
            }
        }
    }
    None
}

/// Compute the maximum effective Broker & Steward skill across EVERY member of the player's fleet:
/// Player Character, all Hired Crew (including Robot crew & Neural/Wafer Jack Expert software),
/// AND all owned Robot/Drone/Software equipment carried by characters or aboard fleet ships.
pub fn compute_fleet_max_broker(
    roster: &[(&crate::character::Character, &[serde_json::Value])],
    fleet_assets: &[serde_json::Value],
) -> FleetBrokerSummary {
    let mut best_broker = 0;
    let mut broker_source = "Default (Untrained 0)".to_string();
    let mut best_steward = 0;
    let mut steward_source = "Default (Untrained 0)".to_string();
    let mut contributors: Vec<(String, i32)> = Vec::new();

    for &(ch, equip) in roster {
        // Only count Player + Hired Crew (not unrelated story NPCs!)
        if ch.kind == crate::character::CharacterKind::Npc {
            continue;
        }
        let (b_lvl, b_src) = effective_skill_with_augments(ch, "Broker");
        contributors.push((b_src.clone(), b_lvl));
        if b_lvl > best_broker {
            best_broker = b_lvl;
            broker_source = b_src;
        }

        let (s_lvl, s_src) = effective_skill_with_augments(ch, "Steward");
        if s_lvl > best_steward {
            best_steward = s_lvl;
            steward_source = s_src;
        }

        // Check robots / broker drones / software in character equipment
        for item in equip {
            if let Some((src, lvl)) = extract_item_skill(item, "Broker") {
                contributors.push((src.clone(), lvl));
                if lvl > best_broker {
                    best_broker = lvl;
                    broker_source = src;
                }
            }
            if let Some((src, lvl)) = extract_item_skill(item, "Steward")
                && lvl > best_steward
            {
                best_steward = lvl;
                steward_source = src;
            }
        }
    }

    // Check robots / shipboard trade bots stored in ship cargo or fleet assets
    for asset in fleet_assets {
        if let Some((src, lvl)) = extract_item_skill(asset, "Broker") {
            contributors.push((src.clone(), lvl));
            if lvl > best_broker {
                best_broker = lvl;
                broker_source = src;
            }
        }
        if let Some((src, lvl)) = extract_item_skill(asset, "Steward")
            && lvl > best_steward
        {
            best_steward = lvl;
            steward_source = src;
        }
    }

    FleetBrokerSummary {
        max_broker_skill: best_broker,
        broker_source,
        max_steward_skill: best_steward,
        steward_source,
        contributors,
    }
}
