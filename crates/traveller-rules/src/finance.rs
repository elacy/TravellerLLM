use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountKind {
    Personal,
    Company,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub kind: AccountKind,
    pub name: String,
    pub owner_character_id: Option<Uuid>,
    pub balance_cr: i64,
    pub credit_limit_cr: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunwayForecast {
    pub company_balance_cr: i64,
    pub personal_balance_cr: i64,
    pub monthly_burn_cr: i64,
    pub months_of_runway: f64,
}

pub fn compute_runway(
    company_balance_cr: i64,
    personal_balance_cr: i64,
    monthly_burn_cr: i64,
) -> RunwayForecast {
    let months_of_runway = if monthly_burn_cr <= 0 {
        f64::INFINITY
    } else {
        (company_balance_cr as f64) / (monthly_burn_cr as f64)
    };
    RunwayForecast {
        company_balance_cr,
        personal_balance_cr,
        monthly_burn_cr,
        months_of_runway,
    }
}
