use rand::Rng;
use serde::{Deserialize, Serialize};

/// Standard Third Imperium calendar: 365 days/year (Day 001 is Holiday), 24h/day, 60m/hour.
/// Every 4 weeks (28 days) is one Maintenance Period (Core 2022 p. 153).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ImperialDate {
    pub year: i32,
    pub day: u16,   // 1..=365
    pub hour: u8,   // 0..=23
    pub minute: u8, // 0..=59
}

impl Default for ImperialDate {
    fn default() -> Self {
        Self {
            year: 1105,
            day: 1,
            hour: 8,
            minute: 0,
        }
    }
}

impl ImperialDate {
    pub fn new(year: i32, day: u16, hour: u8, minute: u8) -> Self {
        let mut d = Self {
            year,
            day: day.clamp(1, 365),
            hour: hour.min(23),
            minute: minute.min(59),
        };
        d.normalize();
        d
    }

    /// Total absolute minutes since Day 001 of Year 0.
    pub fn to_total_minutes(self) -> i64 {
        let total_days = i64::from(self.year) * 365 + i64::from(self.day - 1);
        total_days * 1440 + i64::from(self.hour) * 60 + i64::from(self.minute)
    }

    /// Total absolute days since Day 001 of Year 0.
    pub fn to_total_days(self) -> i64 {
        i64::from(self.year) * 365 + i64::from(self.day - 1)
    }

    pub fn from_total_minutes(total_minutes: i64) -> Self {
        let total_days = total_minutes.div_euclid(1440);
        let rem_mins = total_minutes.rem_euclid(1440) as u32;
        let year = total_days.div_euclid(365) as i32;
        let day = (total_days.rem_euclid(365) as u16) + 1;
        let hour = (rem_mins / 60) as u8;
        let minute = (rem_mins % 60) as u8;
        Self {
            year,
            day,
            hour,
            minute,
        }
    }

    fn normalize(&mut self) {
        *self = Self::from_total_minutes(self.to_total_minutes());
    }

    pub fn advance_minutes(self, delta_minutes: i64) -> Self {
        Self::from_total_minutes(self.to_total_minutes() + delta_minutes)
    }

    pub fn advance_hours(self, hours: i64) -> Self {
        self.advance_minutes(hours * 60)
    }

    pub fn advance_days(self, days: i64) -> Self {
        self.advance_minutes(days * 1440)
    }

    /// Formatted Imperial date string, e.g. `142-1105 14:30`.
    pub fn format(self) -> String {
        format!(
            "{:03}-{:04} {:02}:{:02}",
            self.day, self.year, self.hour, self.minute
        )
    }

    pub fn format_date_only(self) -> String {
        format!("{:03}-{:04}", self.day, self.year)
    }

    /// Maintenance Period index (each Maintenance Period = 4 weeks = 28 days, Core 2022 p. 153).
    pub fn maintenance_period_index(self) -> i64 {
        self.to_total_days().div_euclid(28)
    }

    /// Days remaining until the next 28-day Maintenance Period boundary.
    pub fn days_until_next_maintenance(self) -> i64 {
        28 - self.to_total_days().rem_euclid(28)
    }
}

/// Core 2022 Jump Duration: 168 hours ± 6D6 hours (148 + 6D6 hours gives ~169h, or exact 148 + 6D6
/// / 168 + (3D6 - 3D6)). Core 2022 p. 149: "148 + 6D hours" (~169h average, range 154–184h).
pub fn roll_jump_duration_hours<R: Rng + ?Sized>(rng: &mut R) -> i64 {
    let six_d6: i64 = (0..6).map(|_| rng.random_range(1..=6)).sum();
    148 + six_d6
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeAdvanceSummary {
    pub from: ImperialDate,
    pub to: ImperialDate,
    pub elapsed_minutes: i64,
    pub maintenance_periods_crossed: i64,
    pub annual_boundaries_crossed: i64,
}

pub fn compute_advance(from: ImperialDate, delta_minutes: i64) -> TimeAdvanceSummary {
    let to = from.advance_minutes(delta_minutes);
    let maintenance_periods_crossed =
        (to.maintenance_period_index() - from.maintenance_period_index()).max(0);
    let annual_boundaries_crossed = (i64::from(to.year) - i64::from(from.year)).max(0);
    TimeAdvanceSummary {
        from,
        to,
        elapsed_minutes: delta_minutes,
        maintenance_periods_crossed,
        annual_boundaries_crossed,
    }
}
