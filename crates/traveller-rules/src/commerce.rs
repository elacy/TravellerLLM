use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::dice::{resolve_task, roll_nd6, DiceRollResult, Difficulty, ModifierItem, RollMode};
use crate::universe::Uwp;

/// CSC Update 2023 pp. 5–7 Black Market & Legality Categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegalityCategory {
    Category1Unrestricted,
    Category2Civilian,
    Category3Paramilitary,
    Category4Military,
    Category5RestrictedMilitary,
    Prohibited,
}

impl LegalityCategory {
    pub fn black_market_dm(self) -> i32 {
        match self {
            Self::Category1Unrestricted => 4,
            Self::Category2Civilian => 2,
            Self::Category3Paramilitary => 0,
            Self::Category4Military => -2,
            Self::Category5RestrictedMilitary => -4,
            Self::Prohibited => -6,
        }
    }

    pub fn black_market_price_multiplier(self) -> i64 {
        match self {
            Self::Category1Unrestricted => 2,
            Self::Category2Civilian => 3,
            Self::Category3Paramilitary => 5,
            Self::Category4Military => 10,
            Self::Category5RestrictedMilitary | Self::Prohibited => 20,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityInput {
    pub item_name: String,
    pub item_tl: u8,
    pub base_cost_cr: i64,
    pub specialized: bool,
    pub military_use: bool,
    pub category: LegalityCategory,
    pub black_market: bool,
    pub willing_price_multiplier: u8, // 1, 2, or 3 (legal market)
    pub previous_attempts_this_month: u32,
    pub skill_dm: i32, // Broker or Streetwise (Black market MUST use Streetwise)
    pub stat_dm: i32,  // INT / SOC / EDU DM
    pub referee_dm: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LawResponse {
    None,
    Check,
    Investigate,
    Apprehend,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityResult {
    pub available: bool,
    pub final_unit_price_cr: i64,
    pub roll: DiceRollResult,
    pub law_response: LawResponse,
    pub law_response_roll: Option<i32>,
    pub note: String,
}

/// Exact CSC Update 2023 pp. 4–7 Availability, Black Market & Law Enforcement Response check.
pub fn check_item_availability<R: Rng + ?Sized>(
    rng: &mut R,
    world: &Uwp,
    input: &AvailabilityInput,
) -> AvailabilityResult {
    if world.population == 0 {
        return AvailabilityResult {
            available: false,
            final_unit_price_cr: input.base_cost_cr,
            roll: resolve_task(rng, RollMode::Normal, Difficulty::Impossible, vec![]),
            law_response: LawResponse::None,
            law_response_roll: None,
            note: "Population 0 world: items are unavailable commercially.".into(),
        };
    }

    let mut mods = vec![
        ModifierItem { label: "Stat DM".into(), value: input.stat_dm },
        ModifierItem {
            label: if input.black_market { "Streetwise" } else { "Broker/Streetwise" }.into(),
            value: input.skill_dm,
        },
    ];

    if input.specialized {
        mods.push(ModifierItem { label: "Highly specialised".into(), value: -1 });
    }
    if input.military_use {
        mods.push(ModifierItem { label: "Military use".into(), value: -2 });
    }

    // CSC p. 4 TL difference penalties
    let tl_over = i32::from(input.item_tl) - i32::from(world.tech_level);
    if tl_over >= 10 {
        mods.push(ModifierItem { label: "TL 10+ above world".into(), value: -4 });
    } else if tl_over >= 5 {
        mods.push(ModifierItem { label: "TL 5–9 above world".into(), value: -2 });
    } else if tl_over >= 3 {
        mods.push(ModifierItem { label: "TL 3–4 above world".into(), value: -1 });
    } else if tl_over > 0 {
        mods.push(ModifierItem { label: "TL above world".into(), value: -1 });
    }

    if !input.black_market {
        if input.willing_price_multiplier >= 3 {
            mods.push(ModifierItem { label: "Paying 3x price".into(), value: 2 });
        } else if input.willing_price_multiplier == 2 {
            mods.push(ModifierItem { label: "Paying 2x price".into(), value: 1 });
        }
    }

    match world.starport {
        'A' | 'B' => mods.push(ModifierItem { label: "Starport A/B".into(), value: 1 }),
        'X' => mods.push(ModifierItem { label: "Starport X".into(), value: -4 }),
        _ => {}
    }

    let has_code = |codes: &[&str]| {
        world
            .trade_codes
            .iter()
            .any(|c| codes.iter().any(|x| c.eq_ignore_ascii_case(x)))
    };
    if has_code(&["Hi", "Ht", "In", "Ri"]) {
        mods.push(ModifierItem { label: "Hi/Ht/In/Ri world".into(), value: 2 });
    }
    if has_code(&["Lt", "Na", "Ni", "Po"]) {
        mods.push(ModifierItem { label: "Lt/Na/Ni/Po world".into(), value: -2 });
    }

    match world.population {
        1..=2 => mods.push(ModifierItem { label: "Pop 1–2".into(), value: -2 }),
        3..=5 => mods.push(ModifierItem { label: "Pop 3–5".into(), value: -1 }),
        9 => mods.push(ModifierItem { label: "Pop 9".into(), value: 1 }),
        10..=20 => mods.push(ModifierItem { label: "Pop 10+".into(), value: 2 }),
        _ => {}
    }

    if input.previous_attempts_this_month > 0 {
        mods.push(ModifierItem {
            label: "Previous attempts this month".into(),
            value: -(input.previous_attempts_this_month as i32),
        });
    }

    if input.black_market {
        let law_dm = match world.law_level {
            0 => 2,
            1..=3 => 1,
            4..=6 => 0,
            7..=9 => -1,
            _ => -2,
        };
        mods.push(ModifierItem { label: format!("Black Market Law {}", world.law_level), value: law_dm });
        mods.push(ModifierItem {
            label: format!("Black Market {:?}", input.category),
            value: input.category.black_market_dm(),
        });
    }

    if input.referee_dm != 0 {
        mods.push(ModifierItem { label: "Referee DM".into(), value: input.referee_dm.clamp(-3, 3) });
    }

    let roll = resolve_task(rng, RollMode::Normal, Difficulty::Average, mods);

    let price_mult = if input.black_market {
        input.category.black_market_price_multiplier()
    } else {
        i64::from(input.willing_price_multiplier.max(1))
    };
    let final_unit_price_cr = input.base_cost_cr * price_mult;

    // CSC p. 7: Black market Effect -2 or worse triggers Law Enforcement Response (2D - Effect - Law Level)
    let (law_response, law_response_roll) = if input.black_market && roll.effect <= -2 {
        let d = roll_nd6(rng, 2);
        // Book formula: "deduct both the Effect (apply it as a negative) and the system's Law Level"
        // e.g. Effect -3 means deducting 3 (adding Effect), minus Law Level
        let score = i32::from(d[0]) + i32::from(d[1]) + roll.effect - i32::from(world.law_level);
        let resp = if score <= -6 {
            LawResponse::Apprehend
        } else if score <= -2 {
            LawResponse::Investigate
        } else {
            LawResponse::Check
        };
        (resp, Some(score))
    } else {
        (LawResponse::None, None)
    };

    let note = if roll.success {
        format!(
            "{} located ({}) for Cr{} each.",
            input.item_name,
            if input.black_market { "Black Market" } else { "Legal Market" },
            final_unit_price_cr
        )
    } else if law_response != LawResponse::None {
        format!(
            "Failed to source {} — authorities alerted ({:?})!",
            input.item_name, law_response
        )
    } else {
        format!(
            "{} not found on {} (retry next week at DM-1).",
            input.item_name, world.raw
        )
    };

    AvailabilityResult {
        available: roll.success,
        final_unit_price_cr,
        roll,
        law_response,
        law_response_roll,
        note,
    }
}
