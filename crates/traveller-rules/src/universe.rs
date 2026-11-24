use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UwpError {
    #[error("Invalid UWP '{0}': expected format like 'A788899-C'")]
    InvalidFormat(String),
    #[error("Invalid eHex digit '{0}' in UWP")]
    InvalidEhex(char),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Uwp {
    pub raw: String,
    pub starport: char,
    pub size: u8,
    pub atmosphere: u8,
    pub hydrographics: u8,
    pub population: u8,
    pub government: u8,
    pub law_level: u8,
    pub tech_level: u8,
    pub trade_codes: Vec<String>,
}

pub fn parse_ehex(c: char) -> Result<u8, UwpError> {
    match c.to_ascii_uppercase() {
        '0'..='9' => Ok((c as u8) - b'0'),
        'A'..='H' => Ok(10 + (c.to_ascii_uppercase() as u8 - b'A')),
        'J'..='N' => Ok(18 + (c.to_ascii_uppercase() as u8 - b'J')),
        'P'..='Z' => Ok(23 + (c.to_ascii_uppercase() as u8 - b'P')),
        _ => Err(UwpError::InvalidEhex(c)),
    }
}

/// Parse a MgT2e UWP string (e.g. `A788899-C`) and derive canonical Trade Codes (Core 2022 p. 256).
pub fn parse_uwp(raw: &str) -> Result<Uwp, UwpError> {
    let cleaned = raw.trim().to_ascii_uppercase();
    let chars: Vec<char> = cleaned.chars().collect();
    if chars.len() != 9 || chars[7] != '-' {
        return Err(UwpError::InvalidFormat(raw.to_string()));
    }
    let starport = chars[0];
    let size = parse_ehex(chars[1])?;
    let atmosphere = parse_ehex(chars[2])?;
    let hydrographics = parse_ehex(chars[3])?;
    let population = parse_ehex(chars[4])?;
    let government = parse_ehex(chars[5])?;
    let law_level = parse_ehex(chars[6])?;
    let tech_level = parse_ehex(chars[8])?;

    let mut codes = Vec::new();
    if (4..=9).contains(&atmosphere) && (4..=8).contains(&hydrographics) && (5..=7).contains(&population) {
        codes.push("Ag".into());
    }
    if size == 0 && atmosphere == 0 && hydrographics == 0 {
        codes.push("As".into());
    }
    if population == 0 && government == 0 && law_level == 0 {
        codes.push("Ba".into());
    }
    if (2..=9).contains(&atmosphere) && hydrographics == 0 {
        codes.push("De".into());
    }
    if atmosphere >= 10 && hydrographics >= 1 {
        codes.push("Fl".into());
    }
    if (6..=8).contains(&size) && matches!(atmosphere, 5 | 6 | 8) && (5..=7).contains(&hydrographics) {
        codes.push("Ga".into());
    }
    if population >= 9 {
        codes.push("Hi".into());
    }
    if tech_level >= 12 {
        codes.push("Ht".into());
    }
    if atmosphere <= 1 && hydrographics >= 1 {
        codes.push("Ic".into());
    }
    if matches!(atmosphere, 0 | 1 | 2 | 4 | 7 | 9 | 10 | 11 | 12) && population >= 9 {
        codes.push("In".into());
    }
    if (1..=3).contains(&population) {
        codes.push("Lo".into());
    }
    if tech_level <= 5 && population >= 1 {
        codes.push("Lt".into());
    }
    if atmosphere <= 3 && hydrographics <= 3 && population >= 6 {
        codes.push("Na".into());
    }
    if (4..=6).contains(&population) {
        codes.push("Ni".into());
    }
    if (2..=5).contains(&atmosphere) && hydrographics <= 3 {
        codes.push("Po".into());
    }
    if matches!(atmosphere, 6 | 8) && (6..=8).contains(&population) && (4..=9).contains(&government) {
        codes.push("Ri".into());
    }
    if atmosphere == 0 {
        codes.push("Va".into());
    }
    if ((3..=9).contains(&atmosphere) || atmosphere >= 13) && hydrographics >= 10 {
        codes.push("Wa".into());
    }

    Ok(Uwp {
        raw: cleaned,
        starport,
        size,
        atmosphere,
        hydrographics,
        population,
        government,
        law_level,
        tech_level,
        trade_codes: codes,
    })
}

/// Parsec distance between two TravellerMap hexes `(x1, y1)` and `(x2, y2)` in global odd-q offset coordinates.
pub fn hex_distance(x1: i32, y1: i32, x2: i32, y2: i32) -> i32 {
    let to_cube = |col: i32, row: i32| -> (i32, i32, i32) {
        let q = col;
        let r = row - (col - (col & 1)) / 2;
        let s = -q - r;
        (q, r, s)
    };
    let (q1, r1, s1) = to_cube(x1, y1);
    let (q2, r2, s2) = to_cube(x2, y2);
    ((q1 - q2).abs() + (r1 - r2).abs() + (s1 - s2).abs()) / 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_regina_uwp_and_trade_codes() {
        let regina = parse_uwp("A788899-C").unwrap();
        assert_eq!(regina.starport, 'A');
        assert_eq!(regina.tech_level, 12);
        assert!(regina.trade_codes.contains(&"Ri".to_string()));
        assert!(regina.trade_codes.contains(&"Ht".to_string()));
        assert!(!regina.trade_codes.contains(&"Ga".to_string())); // Hydro 8 > 7
    }

    #[test]
    fn hex_distance_matches_traveller_subsector_geometry() {
        assert_eq!(hex_distance(19, 10, 19, 10), 0);
        assert_eq!(hex_distance(19, 10, 19, 11), 1);
        assert_eq!(hex_distance(19, 10, 20, 10), 1);
    }
}
