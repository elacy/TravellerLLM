use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::dice::{Difficulty, ModifierItem};

#[derive(Debug, Error)]
pub enum CharacterError {
    #[error("SOC cannot be trained with Experience Points (Traveller Companion 2024 p. 43)")]
    SocNotTrainable,
    #[error("Not enough XP: need {needed}, have {available}")]
    InsufficientXp { needed: u32, available: u32 },
    #[error("Skill augmentation requires possessing '{0}' at level 0 or higher")]
    SkillAugmentRequiresSkill(String),
    #[error("Maximum of 3 different skill augmentations may be installed at once")]
    TooManySkillAugments,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharacterKind {
    Player,
    Crew,
    Npc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Characteristic {
    Str,
    Dex,
    End,
    Int,
    Edu,
    Soc,
    Psi,
}

impl Characteristic {
    pub fn is_mental(self) -> bool {
        matches!(self, Characteristic::Int | Characteristic::Edu)
    }

    pub fn is_physical(self) -> bool {
        matches!(
            self,
            Characteristic::Str | Characteristic::Dex | Characteristic::End
        )
    }
}

/// MgT2e Characteristic DM table (Core Rulebook Update 2022)
pub fn characteristic_dm(score: i32) -> i32 {
    match score {
        i32::MIN..=0 => -3,
        1..=2 => -2,
        3..=5 => -1,
        6..=8 => 0,
        9..=11 => 1,
        12..=14 => 2,
        15..=17 => 3,
        18..=20 => 4,
        _ => (score / 3) - 2,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Upp {
    pub str_val: i32,
    pub dex_val: i32,
    pub end_val: i32,
    pub int_val: i32,
    pub edu_val: i32,
    pub soc_val: i32,
    #[serde(default)]
    pub psi_val: Option<i32>,
}

impl Default for Upp {
    fn default() -> Self {
        Self {
            str_val: 7,
            dex_val: 7,
            end_val: 7,
            int_val: 7,
            edu_val: 7,
            soc_val: 7,
            psi_val: None,
        }
    }
}

impl Upp {
    pub fn get(&self, c: Characteristic) -> i32 {
        match c {
            Characteristic::Str => self.str_val,
            Characteristic::Dex => self.dex_val,
            Characteristic::End => self.end_val,
            Characteristic::Int => self.int_val,
            Characteristic::Edu => self.edu_val,
            Characteristic::Soc => self.soc_val,
            Characteristic::Psi => self.psi_val.unwrap_or(0),
        }
    }

    pub fn set(&mut self, c: Characteristic, value: i32) {
        match c {
            Characteristic::Str => self.str_val = value,
            Characteristic::Dex => self.dex_val = value,
            Characteristic::End => self.end_val = value,
            Characteristic::Int => self.int_val = value,
            Characteristic::Edu => self.edu_val = value,
            Characteristic::Soc => self.soc_val = value,
            Characteristic::Psi => self.psi_val = Some(value),
        }
    }

    pub fn dm(&self, c: Characteristic) -> i32 {
        characteristic_dm(self.get(c))
    }

    /// Standard 6-character UPP hex string (e.g. `789A76`).
    pub fn hex_code(&self) -> String {
        let to_ehex = |v: i32| -> char {
            const DIGITS: &[u8] = b"0123456789ABCDEFGHJKLMNPQRSTUVWXYZ";
            let idx = v.clamp(0, (DIGITS.len() - 1) as i32) as usize;
            DIGITS[idx] as char
        };
        [
            self.str_val,
            self.dex_val,
            self.end_val,
            self.int_val,
            self.edu_val,
            self.soc_val,
        ]
        .into_iter()
        .map(to_ehex)
        .collect()
    }
}

/// Expert software package (Core 2022 p. 112–113):
/// - Expert/1 (TL11, BW 1): max Difficult (10+)
/// - Expert/2 (TL12, BW 2): max Very Difficult (12+)
/// - Expert/3 (TL13, BW 3): max Formidable (14+)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpertSoftware {
    pub skill: String,
    pub rating: u8, // 1, 2, or 3 (equals Bandwidth)
    #[serde(default = "default_true")]
    pub active: bool,
}

fn default_true() -> bool {
    true
}

impl ExpertSoftware {
    pub fn max_difficulty(&self) -> Difficulty {
        match self.rating {
            1 => Difficulty::Difficult,
            2 => Difficulty::VeryDifficult,
            _ => Difficulty::Formidable,
        }
    }

    pub fn granted_skill_level(&self) -> i32 {
        i32::from(self.rating).saturating_sub(1)
    }
}

/// Cybernetic / neural augments following exact Core 2022 pp. 107-108 & CSC 2023 pp. 60-61 rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Augment {
    /// Core p. 108 / CSC p. 61: Computer/2 (Basic/Improved) or Computer/3 (Advanced) for Expert
    /// programs on INT or EDU tasks only. Built-in Intelligent Interface (0 BW).
    WaferJack {
        tl: u8,
        computer_rating: u8,
        storage_bandwidth: u8,
        #[serde(default)]
        loaded_expert: Vec<ExpertSoftware>,
    },
    /// CSC p. 61 (TL13+, Cr30,000): Neural comm + nervous system access allowing a connected
    /// Wafer Jack or computer running Expert software to grant physical skills too.
    NeuralLink { tl: u8 },
    /// CSC p. 61 (TL13+, Cr50,000): Combines Neural Link + Wafer Jack/8.
    /// Allows Expert programs for BOTH physical (STR/DEX/END) and mental (INT/EDU) characteristics.
    NeuralJack {
        tl: u8,
        computer_rating: u8,
        storage_bandwidth: u8,
        #[serde(default)]
        loaded_expert: Vec<ExpertSoftware>,
    },
    /// CSC p. 62 (TL12, Cr50,000): DM+1 to one specific skill possessed at level 0+ (max 3 installed).
    SkillAugmentation { skill: String },
    /// CSC p. 61 (TL13, Cr225,000 + 50% suit cost): DM+2 to all DEX checks in powered armour / battle dress.
    NervePlugResponseRig { plugged_in: bool },
    /// CSC p. 61 (TL10, Cr50,000): DM+1 to Initiative and Recon checks.
    PeripheralVisionEnhancement,
    /// Core p. 107: Subdermal Armor (+1 TL10 / +3 TL11).
    SubdermalArmour { protection: i32 },
    /// Generic/custom augment from books.
    Other {
        name: String,
        tl: u8,
        effect: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub kind: CharacterKind,
    pub name: String,
    pub species: String,
    pub age: u16,
    pub upp_max: Upp,
    pub upp_current: Upp,
    /// Skill name -> level (0+). Base skill hierarchy: e.g. having "Gun Combat (Slug): 2"
    /// implies "Gun Combat: 0" for sibling specialties unless broad-skill sub-specialties apply.
    pub skills: BTreeMap<String, i32>,
    #[serde(default)]
    pub augments: Vec<Augment>,
    /// Per-skill dedicated XP (Companion 2024 p. 42 + user preference for per-skill activity XP)
    #[serde(default)]
    pub skill_xp: BTreeMap<String, u32>,
    /// Dedicated XP toward characteristics ("STR", "DEX", "END", "INT", "EDU")
    #[serde(default)]
    pub characteristic_xp: BTreeMap<String, u32>,
    /// General XP pool (awarded at adventure milestones)
    #[serde(default)]
    pub general_xp: u32,
    #[serde(default)]
    pub salary_cr: i64,
    #[serde(default)]
    pub loyalty: i32,
    #[serde(default)]
    pub assigned_ship_id: Option<Uuid>,
    #[serde(default)]
    pub assigned_station: Option<String>,
    #[serde(default)]
    pub notes: String,
}

/// Companion Update 2024 p. 43: Gaining Skills cost table.
/// Level 0: 1 XP, Level 1: 1 XP, Level 2: 2 XP, Level 3: 4 XP, Level 4: 8 XP, Level 5: 16 XP...
pub fn skill_upgrade_cost(target_level: i32) -> u32 {
    if target_level <= 1 {
        1
    } else {
        1u32 << (target_level - 1)
    }
}

/// Companion Update 2024 p. 43: Gaining Characteristics cost table.
/// STR/DEX/END: cost = new_level. INT/EDU: cost = 2 * new_level. SOC: not trainable.
pub fn characteristic_upgrade_cost(c: Characteristic, new_level: i32) -> Result<u32, CharacterError> {
    if c == Characteristic::Soc {
        return Err(CharacterError::SocNotTrainable);
    }
    let lvl = new_level.max(1) as u32;
    if c.is_mental() {
        Ok(lvl * 2)
    } else {
        Ok(lvl)
    }
}

impl Character {
    /// Look up raw skill level (returns None if unskilled -> Jack-of-all-Trades / -3).
    /// Handles specialty fallback: if character has `Pilot (Small Craft): 2`, asking for
    /// `Pilot (Spacecraft)` returns `Some(0)` per Core 2022 skill specialty rules.
    pub fn raw_skill_level(&self, skill: &str) -> Option<i32> {
        if let Some(&lvl) = self.skills.get(skill) {
            return Some(lvl);
        }
        // Case-insensitive exact match
        let lower = skill.to_lowercase();
        for (k, &v) in &self.skills {
            if k.to_lowercase() == lower {
                return Some(v);
            }
        }
        // Specialty base fallback: "Skill (Spec)" -> base "Skill"
        let base = skill.split('(').next().unwrap_or(skill).trim().to_lowercase();
        for (k, &v) in &self.skills {
            let k_base = k.split('(').next().unwrap_or(k).trim().to_lowercase();
            if k_base == base && v >= 0 {
                return Some(0);
            }
        }
        None
    }

    pub fn jack_of_all_trades(&self) -> i32 {
        self.raw_skill_level("Jack-of-All-Trades")
            .or_else(|| self.raw_skill_level("Jack of All Trades"))
            .unwrap_or(0)
            .clamp(0, 3)
    }

    /// Build the exact MgT2e modifier breakdown for a skill + characteristic task check,
    /// including current wounded Characteristic DM, Skill / Unskilled (-3 + JoAT),
    /// Wafer Jack / Neural Link / Neural Jack + Expert Software (Core p.112 & CSC p.61),
    /// Skill Augmentation (CSC p.62), Nerve-plug Response Rig, and Peripheral Vision.
    pub fn task_modifiers(
        &self,
        skill: Option<&str>,
        characteristic: Characteristic,
        difficulty: Difficulty,
    ) -> Vec<ModifierItem> {
        let mut mods = Vec::new();

        // 1. Current (possibly wounded) Characteristic DM
        let stat_val = self.upp_current.get(characteristic);
        let stat_dm = characteristic_dm(stat_val);
        mods.push(ModifierItem {
            label: format!("{characteristic:?} ({stat_val})"),
            value: stat_dm,
        });

        let has_neural_link = self
            .augments
            .iter()
            .any(|a| matches!(a, Augment::NeuralLink { .. } | Augment::NeuralJack { .. }));

        // 2. Skill level + Expert software (Wafer Jack / Neural Jack)
        if let Some(skill_name) = skill {
            let raw_lvl = self.raw_skill_level(skill_name);

            // Check if any active Expert software in a Wafer Jack or Neural Jack applies
            let mut best_expert: Option<(&ExpertSoftware, &'static str)> = None;
            for aug in &self.augments {
                let (source_label, allows_physical, loaded) = match aug {
                    Augment::NeuralJack { loaded_expert, .. } => ("Neural Jack", true, loaded_expert),
                    Augment::WaferJack { loaded_expert, .. } => {
                        ("Wafer Jack", has_neural_link, loaded_expert)
                    }
                    _ => continue,
                };
                if characteristic.is_physical() && !allows_physical {
                    continue;
                }
                if !characteristic.is_mental() && !characteristic.is_physical() {
                    continue;
                }
                for sw in loaded {
                    if !sw.active {
                        continue;
                    }
                    if sw.skill.eq_ignore_ascii_case(skill_name)
                        && difficulty.target() <= sw.max_difficulty().target()
                        && best_expert.is_none_or(|(b, _)| sw.rating > b.rating)
                    {
                        best_expert = Some((sw, source_label));
                    }
                }
            }

            match (raw_lvl, best_expert) {
                (Some(lvl), Some((sw, src))) => {
                    mods.push(ModifierItem {
                        label: format!("{skill_name} {lvl}"),
                        value: lvl,
                    });
                    // Core 2022 p. 112: If Traveller already has the skill, Expert grants DM+1.
                    mods.push(ModifierItem {
                        label: format!("{src} (Expert/{} assist)", sw.rating),
                        value: 1,
                    });
                }
                (Some(lvl), None) => {
                    mods.push(ModifierItem {
                        label: format!("{skill_name} {lvl}"),
                        value: lvl,
                    });
                }
                (None, Some((sw, src))) => {
                    // Core 2022 p. 112: Unskilled Traveller uses Expert at Bandwidth - 1.
                    let granted = sw.granted_skill_level();
                    mods.push(ModifierItem {
                        label: format!("{src} (Expert/{} -> {skill_name} {granted})", sw.rating),
                        value: granted,
                    });
                }
                (None, None) => {
                    let joat = self.jack_of_all_trades();
                    let pen = (-3 + joat).min(0);
                    mods.push(ModifierItem {
                        label: if joat > 0 {
                            format!("Unskilled (JoAT-{joat})")
                        } else {
                            "Unskilled".to_string()
                        },
                        value: pen,
                    });
                }
            }

            // 3. Skill Augmentation (CSC p. 62): +1 if possessing skill at 0+
            if raw_lvl.is_some()
                && self.augments.iter().any(|a| {
                    matches!(a, Augment::SkillAugmentation { skill: s } if s.eq_ignore_ascii_case(skill_name))
                })
            {
                mods.push(ModifierItem {
                    label: format!("Skill Augmentation ({skill_name})"),
                    value: 1,
                });
            }

            // 4. Peripheral Vision Enhancement on Recon
            if skill_name.eq_ignore_ascii_case("Recon")
                && self
                    .augments
                    .iter()
                    .any(|a| matches!(a, Augment::PeripheralVisionEnhancement))
            {
                mods.push(ModifierItem {
                    label: "Peripheral Vision Enhancement".to_string(),
                    value: 1,
                });
            }
        }

        // 5. Nerve-plug Response Rig: DM+2 to all DEX checks when plugged into powered armour
        if characteristic == Characteristic::Dex
            && self
                .augments
                .iter()
                .any(|a| matches!(a, Augment::NervePlugResponseRig { plugged_in: true }))
        {
            mods.push(ModifierItem {
                label: "Nerve-plug Response Rig".to_string(),
                value: 2,
            });
        }

        mods
    }

    /// Award per-skill dedicated XP from using a skill in play (Companion 2024).
    pub fn award_skill_xp(&mut self, skill: &str, amount: u32) -> u32 {
        let entry = self.skill_xp.entry(skill.to_string()).or_insert(0);
        *entry += amount;
        *entry
    }

    /// Upgrade a skill by 1 level using dedicated skill XP first, then optionally general XP.
    pub fn upgrade_skill(&mut self, skill: &str, allow_general_xp: bool) -> Result<i32, CharacterError> {
        let current = self.skills.get(skill).copied();
        let target_level = match current {
            None => 0,
            Some(lvl) => lvl + 1,
        };
        let cost = skill_upgrade_cost(target_level);
        let dedicated = self.skill_xp.get(skill).copied().unwrap_or(0);
        let available = if allow_general_xp {
            dedicated + self.general_xp
        } else {
            dedicated
        };

        if available < cost {
            return Err(CharacterError::InsufficientXp {
                needed: cost,
                available,
            });
        }

        let from_dedicated = dedicated.min(cost);
        let from_general = cost - from_dedicated;
        *self.skill_xp.entry(skill.to_string()).or_insert(0) -= from_dedicated;
        self.general_xp -= from_general;
        self.skills.insert(skill.to_string(), target_level);
        Ok(target_level)
    }

    /// Upgrade a characteristic by +1 using Companion 2024 costs.
    pub fn upgrade_characteristic(
        &mut self,
        c: Characteristic,
        allow_general_xp: bool,
    ) -> Result<i32, CharacterError> {
        let new_level = self.upp_max.get(c) + 1;
        let cost = characteristic_upgrade_cost(c, new_level)?;
        let key = format!("{c:?}").to_uppercase();
        let dedicated = self.characteristic_xp.get(&key).copied().unwrap_or(0);
        let available = if allow_general_xp {
            dedicated + self.general_xp
        } else {
            dedicated
        };

        if available < cost {
            return Err(CharacterError::InsufficientXp {
                needed: cost,
                available,
            });
        }

        let from_dedicated = dedicated.min(cost);
        let from_general = cost - from_dedicated;
        *self.characteristic_xp.entry(key).or_insert(0) -= from_dedicated;
        self.general_xp -= from_general;
        self.upp_max.set(c, new_level);
        self.upp_current.set(c, self.upp_current.get(c) + 1);
        Ok(new_level)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_character() -> Character {
        let mut skills = BTreeMap::new();
        skills.insert("Astrogation".to_string(), 1);
        Character {
            id: Uuid::new_v4(),
            campaign_id: Uuid::new_v4(),
            kind: CharacterKind::Player,
            name: "Elena Vance".to_string(),
            species: "Human".to_string(),
            age: 30,
            upp_max: Upp {
                str_val: 7,
                dex_val: 9,
                end_val: 8,
                int_val: 10,
                edu_val: 9,
                soc_val: 7,
                psi_val: None,
            },
            upp_current: Upp {
                str_val: 7,
                dex_val: 9,
                end_val: 8,
                int_val: 10,
                edu_val: 9,
                soc_val: 7,
                psi_val: None,
            },
            skills,
            augments: vec![],
            skill_xp: BTreeMap::new(),
            characteristic_xp: BTreeMap::new(),
            general_xp: 0,
            salary_cr: 0,
            loyalty: 0,
            assigned_ship_id: None,
            assigned_station: None,
            notes: String::new(),
        }
    }

    #[test]
    fn wafer_jack_only_allows_int_edu_while_neural_jack_allows_physical() {
        let mut c = sample_character();
        c.augments.push(Augment::WaferJack {
            tl: 13,
            computer_rating: 2,
            storage_bandwidth: 8,
            loaded_expert: vec![
                ExpertSoftware {
                    skill: "Gun Combat (Slug)".to_string(),
                    rating: 2,
                    active: true,
                },
                ExpertSoftware {
                    skill: "Astrogation".to_string(),
                    rating: 2,
                    active: true,
                },
            ],
        });

        // Physical skill with plain Wafer Jack -> still unskilled (-3)
        let phys = c.task_modifiers(Some("Gun Combat (Slug)"), Characteristic::Dex, Difficulty::Average);
        assert!(phys.iter().any(|m| m.label == "Unskilled" && m.value == -3));

        // Mental skill already possessed (Astrogation 1) -> grants DM+1 assist
        let mental = c.task_modifiers(Some("Astrogation"), Characteristic::Edu, Difficulty::Average);
        assert!(mental.iter().any(|m| m.value == 1 && m.label.contains("Expert/2 assist")));

        // Upgrade to Neural Jack (CSC 2023 p. 61) -> physical Expert/2 now grants skill level 1!
        c.augments.clear();
        c.augments.push(Augment::NeuralJack {
            tl: 13,
            computer_rating: 2,
            storage_bandwidth: 8,
            loaded_expert: vec![ExpertSoftware {
                skill: "Gun Combat (Slug)".to_string(),
                rating: 2,
                active: true,
            }],
        });
        let phys_nj = c.task_modifiers(Some("Gun Combat (Slug)"), Characteristic::Dex, Difficulty::Average);
        assert!(phys_nj.iter().any(|m| m.label.contains("Neural Jack") && m.value == 1));
    }

    #[test]
    fn companion_2024_xp_costs_and_per_skill_upgrade() {
        assert_eq!(skill_upgrade_cost(0), 1);
        assert_eq!(skill_upgrade_cost(1), 1);
        assert_eq!(skill_upgrade_cost(2), 2);
        assert_eq!(skill_upgrade_cost(3), 4);
        assert_eq!(skill_upgrade_cost(4), 8);
        assert_eq!(skill_upgrade_cost(5), 16);
        assert_eq!(skill_upgrade_cost(6), 32);

        let mut c = sample_character();
        // Astrogation is currently 1 -> level 2 costs 2 XP
        c.award_skill_xp("Astrogation", 2);
        let new_lvl = c.upgrade_skill("Astrogation", false).unwrap();
        assert_eq!(new_lvl, 2);
        assert_eq!(*c.skill_xp.get("Astrogation").unwrap(), 0);

        // SOC cannot be trained
        assert!(matches!(
            c.upgrade_characteristic(Characteristic::Soc, true),
            Err(CharacterError::SocNotTrainable)
        ));
    }
}
