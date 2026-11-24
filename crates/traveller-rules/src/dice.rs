use rand::Rng;
use serde::{Deserialize, Serialize};

/// MgT2e Task Difficulties (Core Rulebook Update 2022)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Difficulty {
    Simple,        // 2+
    Easy,          // 4+
    Routine,       // 6+
    Average,       // 8+
    Difficult,     // 10+
    VeryDifficult, // 12+
    Formidable,    // 14+
    Impossible,    // 16+
}

impl Difficulty {
    pub fn target(self) -> i32 {
        match self {
            Difficulty::Simple => 2,
            Difficulty::Easy => 4,
            Difficulty::Routine => 6,
            Difficulty::Average => 8,
            Difficulty::Difficult => 10,
            Difficulty::VeryDifficult => 12,
            Difficulty::Formidable => 14,
            Difficulty::Impossible => 16,
        }
    }

    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().replace([' ', '-'], "_").as_str() {
            "simple" | "2" | "2+" => Some(Self::Simple),
            "easy" | "4" | "4+" => Some(Self::Easy),
            "routine" | "6" | "6+" => Some(Self::Routine),
            "average" | "8" | "8+" => Some(Self::Average),
            "difficult" | "10" | "10+" => Some(Self::Difficult),
            "very_difficult" | "verydifficult" | "12" | "12+" => Some(Self::VeryDifficult),
            "formidable" | "14" | "14+" => Some(Self::Formidable),
            "impossible" | "16" | "16+" => Some(Self::Impossible),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RollMode {
    #[default]
    Normal, // 2D6
    Boon, // 3D6 drop lowest
    Bane, // 3D6 drop highest
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifierItem {
    pub label: String,
    pub value: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiceRollResult {
    pub mode: RollMode,
    pub raw_dice: Vec<u8>,
    pub kept_dice: [u8; 2],
    pub natural_sum: i32,
    pub modifiers: Vec<ModifierItem>,
    pub total_dm: i32,
    pub total: i32,
    pub difficulty: Difficulty,
    pub target: i32,
    pub effect: i32,
    pub success: bool,
    pub degree: String,
    pub task_chain_dm: i32,
}

/// MgT2e Task Chain Effect -> DM table (Core Rulebook Update 2022)
pub fn task_chain_dm(effect: i32) -> i32 {
    match effect {
        i32::MIN..=-6 => -3,
        -5..=-2 => -2,
        -1 => -1,
        0 => 0,
        1..=5 => 1,
        6..=i32::MAX => 2,
    }
}

pub fn effect_degree(effect: i32) -> &'static str {
    match effect {
        i32::MIN..=-6 => "Exceptional Failure",
        -5..=-2 => "Average Failure",
        -1 => "Marginal Failure",
        0 => "Marginal Success",
        1..=5 => "Average Success",
        6..=i32::MAX => "Exceptional Success",
    }
}

/// Roll Nd6 using any Rng (for deterministic unit tests and live crypto/thread RNG).
pub fn roll_nd6<R: Rng + ?Sized>(rng: &mut R, count: usize) -> Vec<u8> {
    (0..count).map(|_| rng.random_range(1..=6)).collect()
}

pub fn roll_d66<R: Rng + ?Sized>(rng: &mut R) -> u8 {
    let tens: u8 = rng.random_range(1..=6);
    let ones: u8 = rng.random_range(1..=6);
    tens * 10 + ones
}

/// Resolve a MgT2e task check (2D6, Boon 3D6 drop lowest, Bane 3D6 drop highest).
pub fn resolve_task<R: Rng + ?Sized>(
    rng: &mut R,
    mode: RollMode,
    difficulty: Difficulty,
    modifiers: Vec<ModifierItem>,
) -> DiceRollResult {
    let raw_dice = match mode {
        RollMode::Normal => roll_nd6(rng, 2),
        RollMode::Boon | RollMode::Bane => roll_nd6(rng, 3),
    };

    let kept_dice = match mode {
        RollMode::Normal => [raw_dice[0], raw_dice[1]],
        RollMode::Boon => {
            let mut sorted = raw_dice.clone();
            sorted.sort_unstable();
            [sorted[1], sorted[2]]
        }
        RollMode::Bane => {
            let mut sorted = raw_dice.clone();
            sorted.sort_unstable();
            [sorted[0], sorted[1]]
        }
    };

    let natural_sum = i32::from(kept_dice[0]) + i32::from(kept_dice[1]);
    let total_dm: i32 = modifiers.iter().map(|m| m.value).sum();
    let total = natural_sum + total_dm;
    let target = difficulty.target();
    let effect = total - target;
    let success = effect >= 0;

    DiceRollResult {
        mode,
        raw_dice,
        kept_dice,
        natural_sum,
        modifiers,
        total_dm,
        total,
        difficulty,
        target,
        effect,
        success,
        degree: effect_degree(effect).to_string(),
        task_chain_dm: task_chain_dm(effect),
    }
}
