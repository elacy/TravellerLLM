#![allow(clippy::too_many_arguments)]

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use traveller_rules::calendar::ImperialDate;
use traveller_rules::character::{Character, CharacterKind};
use traveller_rules::finance::AccountKind;
use traveller_rules::ship::{
    compute_running_costs, validate_ship, Mortgage, ShipSpec,
};
use traveller_rules::universe::parse_uwp;
use uuid::Uuid;

use crate::models::*;

#[derive(Clone)]
pub struct Db {
    pool: PgPool,
}

impl Db {
    pub async fn connect(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(12)
            .connect(database_url)
            .await
            .context("Failed to connect to PostgreSQL")?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .context("Failed to run PostgreSQL migrations")?;
        Ok(())
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // ── Campaigns ──────────────────────────────────────────────────────────

    pub async fn list_campaigns(&self) -> Result<Vec<CampaignRecord>> {
        let rows = sqlx::query(
            "SELECT id, name, imperial_year, imperial_day, hour, minute,
                    current_system_name, current_system_uwp, referee_model, worker_model,
                    hindsight_bank_id, story_ledger, created_at, updated_at
             FROM campaigns ORDER BY updated_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(Self::map_campaign).collect()
    }

    pub async fn get_campaign(&self, id: Uuid) -> Result<Option<CampaignRecord>> {
        let row = sqlx::query(
            "SELECT id, name, imperial_year, imperial_day, hour, minute,
                    current_system_name, current_system_uwp, referee_model, worker_model,
                    hindsight_bank_id, story_ledger, created_at, updated_at
             FROM campaigns WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(Self::map_campaign).transpose()
    }

    pub async fn create_campaign(
        &self,
        name: &str,
        system_name: &str,
        system_uwp: &str,
        referee_model: &str,
        worker_model: &str,
    ) -> Result<CampaignRecord> {
        let id = Uuid::new_v4();
        let bank_id = format!("traveller-{id}");
        let row = sqlx::query(
            "INSERT INTO campaigns
                (id, name, current_system_name, current_system_uwp, referee_model, worker_model, hindsight_bank_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING id, name, imperial_year, imperial_day, hour, minute,
                       current_system_name, current_system_uwp, referee_model, worker_model,
                       hindsight_bank_id, story_ledger, created_at, updated_at",
        )
        .bind(id)
        .bind(name)
        .bind(system_name)
        .bind(system_uwp)
        .bind(referee_model)
        .bind(worker_model)
        .bind(&bank_id)
        .fetch_one(&self.pool)
        .await?;

        // Seed default Personal & Company accounts so finances work out of the box
        self.create_account(id, AccountKind::Personal, "Personal Account", None, 25_000, 0)
            .await?;
        self.create_account(id, AccountKind::Company, "Ship Operating Account", None, 250_000, 0)
            .await?;

        Self::map_campaign(row)
    }

    pub async fn update_campaign_date(&self, campaign_id: Uuid, date: ImperialDate) -> Result<()> {
        sqlx::query(
            "UPDATE campaigns
             SET imperial_year = $2, imperial_day = $3, hour = $4, minute = $5, updated_at = NOW()
             WHERE id = $1",
        )
        .bind(campaign_id)
        .bind(date.year)
        .bind(date.day as i16)
        .bind(date.hour as i16)
        .bind(date.minute as i16)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_story_ledger(
        &self,
        campaign_id: Uuid,
        ledger: &StoryLedger,
    ) -> Result<()> {
        sqlx::query("UPDATE campaigns SET story_ledger = $2, updated_at = NOW() WHERE id = $1")
            .bind(campaign_id)
            .bind(serde_json::to_value(ledger)?)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    fn map_campaign(r: sqlx::postgres::PgRow) -> Result<CampaignRecord> {
        let ledger_val: Value = r.try_get("story_ledger")?;
        let story_ledger: StoryLedger = serde_json::from_value(ledger_val).unwrap_or_default();
        Ok(CampaignRecord {
            id: r.try_get("id")?,
            name: r.try_get("name")?,
            date: ImperialDate::new(
                r.try_get("imperial_year")?,
                r.try_get::<i16, _>("imperial_day")? as u16,
                r.try_get::<i16, _>("hour")? as u8,
                r.try_get::<i16, _>("minute")? as u8,
            ),
            current_system_name: r.try_get("current_system_name")?,
            current_system_uwp: r.try_get("current_system_uwp")?,
            referee_model: r.try_get("referee_model")?,
            worker_model: r.try_get("worker_model")?,
            hindsight_bank_id: r.try_get("hindsight_bank_id")?,
            story_ledger,
            created_at: r.try_get("created_at")?,
            updated_at: r.try_get("updated_at")?,
        })
    }

    // ── Characters ─────────────────────────────────────────────────────────

    pub async fn list_characters(&self, campaign_id: Uuid) -> Result<Vec<CharacterRow>> {
        let rows = sqlx::query(
            "SELECT id, campaign_id, kind, name, species, age, upp_max, upp_current,
                    skills, augments, equipment, skill_xp, characteristic_xp, general_xp,
                    salary_cr, loyalty, assigned_ship_id, assigned_station, notes
             FROM characters WHERE campaign_id = $1 ORDER BY kind ASC, name ASC",
        )
        .bind(campaign_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(Self::map_character).collect()
    }

    pub async fn get_character(&self, campaign_id: Uuid, char_id: Uuid) -> Result<Option<CharacterRow>> {
        let row = sqlx::query(
            "SELECT id, campaign_id, kind, name, species, age, upp_max, upp_current,
                    skills, augments, equipment, skill_xp, characteristic_xp, general_xp,
                    salary_cr, loyalty, assigned_ship_id, assigned_station, notes
             FROM characters WHERE campaign_id = $1 AND id = $2",
        )
        .bind(campaign_id)
        .bind(char_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(Self::map_character).transpose()
    }

    pub async fn create_character(
        &self,
        campaign_id: Uuid,
        input: NewCharacterInput,
    ) -> Result<CharacterRow> {
        if input.kind == CharacterKind::Player && self.has_player_character(campaign_id, None).await? {
            anyhow::bail!(
                "This campaign already has a Player Character. Each game supports strictly one Player Character (edit or replace the existing Player Character, or add Crew/NPCs instead)."
            );
        }
        let id = Uuid::new_v4();
        let kind_str = match input.kind {
            CharacterKind::Player => "player",
            CharacterKind::Crew => "crew",
            CharacterKind::Npc => "npc",
        };
        let upp_json = serde_json::to_value(&input.upp)?;
        let row = sqlx::query(
            "INSERT INTO characters
                (id, campaign_id, kind, name, species, age, upp_max, upp_current,
                 skills, augments, salary_cr, assigned_ship_id, assigned_station, notes)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
             RETURNING id, campaign_id, kind, name, species, age, upp_max, upp_current,
                       skills, augments, equipment, skill_xp, characteristic_xp, general_xp,
                       salary_cr, loyalty, assigned_ship_id, assigned_station, notes",
        )
        .bind(id)
        .bind(campaign_id)
        .bind(kind_str)
        .bind(&input.name)
        .bind(&input.species)
        .bind(input.age as i16)
        .bind(&upp_json)
        .bind(&upp_json)
        .bind(serde_json::to_value(&input.skills)?)
        .bind(serde_json::to_value(&input.augments)?)
        .bind(input.salary_cr)
        .bind(input.assigned_ship_id)
        .bind(&input.assigned_station)
        .bind(&input.notes)
        .fetch_one(&self.pool)
        .await?;

        Self::map_character(row)
    }

    pub async fn has_player_character(
        &self,
        campaign_id: Uuid,
        exclude_char_id: Option<Uuid>,
    ) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM characters
             WHERE campaign_id = $1 AND kind = 'player' AND ($2::uuid IS NULL OR id <> $2)",
        )
        .bind(campaign_id)
        .bind(exclude_char_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(count > 0)
    }

    pub async fn delete_player_characters(&self, campaign_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM characters WHERE campaign_id = $1 AND kind = 'player'")
            .bind(campaign_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_character(&self, campaign_id: Uuid, char_id: Uuid) -> Result<bool> {
        let res = sqlx::query("DELETE FROM characters WHERE campaign_id = $1 AND id = $2")
            .bind(campaign_id)
            .bind(char_id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn delete_campaign(&self, campaign_id: Uuid) -> Result<bool> {
        let res = sqlx::query("DELETE FROM campaigns WHERE id = $1")
            .bind(campaign_id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn save_character(&self, c: &Character, equipment: &[Value]) -> Result<()> {
        if c.kind == CharacterKind::Player
            && self.has_player_character(c.campaign_id, Some(c.id)).await?
        {
            anyhow::bail!(
                "Cannot change character to Player: this campaign already has a Player Character."
            );
        }
        let kind_str = match c.kind {
            CharacterKind::Player => "player",
            CharacterKind::Crew => "crew",
            CharacterKind::Npc => "npc",
        };
        sqlx::query(
            "UPDATE characters
             SET kind = $3, name = $4, species = $5, age = $6,
                 upp_max = $7, upp_current = $8, skills = $9, augments = $10,
                 equipment = $11, skill_xp = $12, characteristic_xp = $13,
                 general_xp = $14, salary_cr = $15, loyalty = $16,
                 assigned_ship_id = $17, assigned_station = $18, notes = $19,
                 updated_at = NOW()
             WHERE campaign_id = $1 AND id = $2",
        )
        .bind(c.campaign_id)
        .bind(c.id)
        .bind(kind_str)
        .bind(&c.name)
        .bind(&c.species)
        .bind(c.age as i16)
        .bind(serde_json::to_value(&c.upp_max)?)
        .bind(serde_json::to_value(&c.upp_current)?)
        .bind(serde_json::to_value(&c.skills)?)
        .bind(serde_json::to_value(&c.augments)?)
        .bind(serde_json::to_value(equipment)?)
        .bind(serde_json::to_value(&c.skill_xp)?)
        .bind(serde_json::to_value(&c.characteristic_xp)?)
        .bind(c.general_xp as i32)
        .bind(c.salary_cr)
        .bind(c.loyalty)
        .bind(c.assigned_ship_id)
        .bind(&c.assigned_station)
        .bind(&c.notes)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    fn map_character(r: sqlx::postgres::PgRow) -> Result<CharacterRow> {
        let kind_s: String = r.try_get("kind")?;
        let kind = match kind_s.as_str() {
            "player" => CharacterKind::Player,
            "crew" => CharacterKind::Crew,
            _ => CharacterKind::Npc,
        };
        let character = Character {
            id: r.try_get("id")?,
            campaign_id: r.try_get("campaign_id")?,
            kind,
            name: r.try_get("name")?,
            species: r.try_get("species")?,
            age: r.try_get::<i16, _>("age")? as u16,
            upp_max: serde_json::from_value(r.try_get("upp_max")?)?,
            upp_current: serde_json::from_value(r.try_get("upp_current")?)?,
            skills: serde_json::from_value(r.try_get("skills")?)?,
            augments: serde_json::from_value(r.try_get("augments")?)?,
            skill_xp: serde_json::from_value(r.try_get("skill_xp")?)?,
            characteristic_xp: serde_json::from_value(r.try_get("characteristic_xp")?)?,
            general_xp: r.try_get::<i32, _>("general_xp")? as u32,
            salary_cr: r.try_get("salary_cr")?,
            loyalty: r.try_get("loyalty")?,
            assigned_ship_id: r.try_get("assigned_ship_id")?,
            assigned_station: r.try_get("assigned_station")?,
            notes: r.try_get("notes")?,
        };
        let equipment: Vec<Value> = serde_json::from_value(r.try_get("equipment")?)?;
        Ok(CharacterRow {
            character,
            equipment,
        })
    }

    // ── Ships ──────────────────────────────────────────────────────────────

    pub async fn list_ships(&self, campaign_id: Uuid) -> Result<Vec<ShipRecord>> {
        let rows = sqlx::query(
            "SELECT id, campaign_id, name, class_name, owner_account_id, spec, validation,
                    hull_current, fuel_refined, fuel_unrefined, crits, cargo, passengers,
                    mortgage, skipped_maintenance_periods, last_annual_overhaul_year, docked_system_name,
                    is_docked, docked_at_total_minutes, berthing_billed_days
             FROM ships WHERE campaign_id = $1 ORDER BY created_at ASC",
        )
        .bind(campaign_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(Self::map_ship).collect()
    }

    pub async fn create_ship(
        &self,
        campaign_id: Uuid,
        name: &str,
        class_name: &str,
        owner_account_id: Option<Uuid>,
        spec: ShipSpec,
        mortgage: Option<Mortgage>,
        docked_system_name: &str,
    ) -> Result<ShipRecord> {
        let id = Uuid::new_v4();
        let validation = validate_ship(&spec);
        let hull_current = spec.hull_points_max;
        let fuel_refined = spec.fuel_capacity_tons;

        let row = sqlx::query(
            "INSERT INTO ships
                (id, campaign_id, name, class_name, owner_account_id, spec, validation,
                 hull_current, fuel_refined, mortgage, docked_system_name,
                 is_docked, docked_at_total_minutes, berthing_billed_days)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,FALSE,NULL,0)
             RETURNING id, campaign_id, name, class_name, owner_account_id, spec, validation,
                       hull_current, fuel_refined, fuel_unrefined, crits, cargo, passengers,
                       mortgage, skipped_maintenance_periods, last_annual_overhaul_year, docked_system_name,
                       is_docked, docked_at_total_minutes, berthing_billed_days",
        )
        .bind(id)
        .bind(campaign_id)
        .bind(name)
        .bind(class_name)
        .bind(owner_account_id)
        .bind(serde_json::to_value(&spec)?)
        .bind(serde_json::to_value(&validation)?)
        .bind(hull_current)
        .bind(fuel_refined)
        .bind(mortgage.as_ref().map(serde_json::to_value).transpose()?)
        .bind(docked_system_name)
        .fetch_one(&self.pool)
        .await?;

        Self::map_ship(row)
    }

    pub async fn save_ship(&self, ship: &ShipRecord) -> Result<()> {
        let validation = validate_ship(&ship.spec);
        sqlx::query(
            "UPDATE ships
             SET name = $3, class_name = $4, owner_account_id = $5, spec = $6, validation = $7,
                 hull_current = $8, fuel_refined = $9, fuel_unrefined = $10, crits = $11,
                 cargo = $12, passengers = $13, mortgage = $14, skipped_maintenance_periods = $15,
                 last_annual_overhaul_year = $16, docked_system_name = $17,
                 is_docked = $18, docked_at_total_minutes = $19, berthing_billed_days = $20,
                 updated_at = NOW()
             WHERE campaign_id = $1 AND id = $2",
        )
        .bind(ship.campaign_id)
        .bind(ship.id)
        .bind(&ship.name)
        .bind(&ship.class_name)
        .bind(ship.owner_account_id)
        .bind(serde_json::to_value(&ship.spec)?)
        .bind(serde_json::to_value(&validation)?)
        .bind(ship.hull_current)
        .bind(ship.fuel_refined)
        .bind(ship.fuel_unrefined)
        .bind(serde_json::to_value(&ship.crits)?)
        .bind(serde_json::to_value(&ship.cargo)?)
        .bind(serde_json::to_value(&ship.passengers)?)
        .bind(ship.mortgage.as_ref().map(serde_json::to_value).transpose()?)
        .bind(ship.skipped_maintenance_periods)
        .bind(ship.last_annual_overhaul_year)
        .bind(&ship.docked_system_name)
        .bind(ship.is_docked)
        .bind(ship.docked_at_total_minutes)
        .bind(ship.berthing_billed_days)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    fn map_ship(r: sqlx::postgres::PgRow) -> Result<ShipRecord> {
        let mortgage_val: Option<Value> = r.try_get("mortgage")?;
        Ok(ShipRecord {
            id: r.try_get("id")?,
            campaign_id: r.try_get("campaign_id")?,
            name: r.try_get("name")?,
            class_name: r.try_get("class_name")?,
            owner_account_id: r.try_get("owner_account_id")?,
            spec: serde_json::from_value(r.try_get("spec")?)?,
            validation: serde_json::from_value(r.try_get("validation")?)?,
            hull_current: r.try_get("hull_current")?,
            fuel_refined: r.try_get("fuel_refined")?,
            fuel_unrefined: r.try_get("fuel_unrefined")?,
            crits: serde_json::from_value(r.try_get("crits")?)?,
            cargo: serde_json::from_value(r.try_get("cargo")?)?,
            passengers: serde_json::from_value(r.try_get("passengers")?)?,
            mortgage: mortgage_val.map(serde_json::from_value).transpose()?,
            skipped_maintenance_periods: r.try_get("skipped_maintenance_periods")?,
            last_annual_overhaul_year: r.try_get("last_annual_overhaul_year")?,
            docked_system_name: r.try_get("docked_system_name")?,
            is_docked: r.try_get("is_docked")?,
            docked_at_total_minutes: r.try_get("docked_at_total_minutes")?,
            berthing_billed_days: r.try_get("berthing_billed_days")?,
        })
    }

    // ── Accounts & Transactions ────────────────────────────────────────────

    pub async fn list_accounts(&self, campaign_id: Uuid) -> Result<Vec<AccountRecord>> {
        let rows = sqlx::query(
            "SELECT id, campaign_id, kind, name, owner_character_id, balance_cr, credit_limit_cr
             FROM accounts WHERE campaign_id = $1 ORDER BY kind ASC, name ASC",
        )
        .bind(campaign_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| {
                let kind_s: String = r.try_get("kind")?;
                Ok(AccountRecord {
                    id: r.try_get("id")?,
                    campaign_id: r.try_get("campaign_id")?,
                    kind: if kind_s == "company" {
                        AccountKind::Company
                    } else {
                        AccountKind::Personal
                    },
                    name: r.try_get("name")?,
                    owner_character_id: r.try_get("owner_character_id")?,
                    balance_cr: r.try_get("balance_cr")?,
                    credit_limit_cr: r.try_get("credit_limit_cr")?,
                })
            })
            .collect()
    }

    pub async fn create_account(
        &self,
        campaign_id: Uuid,
        kind: AccountKind,
        name: &str,
        owner_character_id: Option<Uuid>,
        balance_cr: i64,
        credit_limit_cr: i64,
    ) -> Result<AccountRecord> {
        let id = Uuid::new_v4();
        let kind_s = match kind {
            AccountKind::Personal => "personal",
            AccountKind::Company => "company",
        };
        sqlx::query(
            "INSERT INTO accounts (id, campaign_id, kind, name, owner_character_id, balance_cr, credit_limit_cr)
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(id)
        .bind(campaign_id)
        .bind(kind_s)
        .bind(name)
        .bind(owner_character_id)
        .bind(balance_cr)
        .bind(credit_limit_cr)
        .execute(&self.pool)
        .await?;

        Ok(AccountRecord {
            id,
            campaign_id,
            kind,
            name: name.to_string(),
            owner_character_id,
            balance_cr,
            credit_limit_cr,
        })
    }

    pub async fn post_transaction(
        &self,
        campaign_id: Uuid,
        account_id: Uuid,
        imperial_date: &str,
        amount_cr: i64,
        category: &str,
        description: &str,
        counterparty: &str,
    ) -> Result<TransactionRecord> {
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            "UPDATE accounts SET balance_cr = balance_cr + $2, updated_at = NOW()
             WHERE id = $1 AND campaign_id = $3
             RETURNING balance_cr",
        )
        .bind(account_id)
        .bind(amount_cr)
        .bind(campaign_id)
        .fetch_one(&mut *tx)
        .await?;
        let balance_after: i64 = row.try_get("balance_cr")?;

        let tx_id = Uuid::new_v4();
        let inserted = sqlx::query(
            "INSERT INTO transactions
                (id, campaign_id, account_id, imperial_date, amount_cr, balance_after_cr, category, description, counterparty)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
             RETURNING created_at",
        )
        .bind(tx_id)
        .bind(campaign_id)
        .bind(account_id)
        .bind(imperial_date)
        .bind(amount_cr)
        .bind(balance_after)
        .bind(category)
        .bind(description)
        .bind(counterparty)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(TransactionRecord {
            id: tx_id,
            campaign_id,
            account_id,
            imperial_date: imperial_date.to_string(),
            amount_cr,
            balance_after_cr: balance_after,
            category: category.to_string(),
            description: description.to_string(),
            counterparty: counterparty.to_string(),
            created_at: inserted.try_get("created_at")?,
        })
    }

    pub async fn list_transactions(
        &self,
        campaign_id: Uuid,
        limit: i64,
    ) -> Result<Vec<TransactionRecord>> {
        let rows = sqlx::query(
            "SELECT id, campaign_id, account_id, imperial_date, amount_cr, balance_after_cr,
                    category, description, counterparty, created_at
             FROM transactions WHERE campaign_id = $1 ORDER BY created_at DESC LIMIT $2",
        )
        .bind(campaign_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| {
                Ok(TransactionRecord {
                    id: r.try_get("id")?,
                    campaign_id: r.try_get("campaign_id")?,
                    account_id: r.try_get("account_id")?,
                    imperial_date: r.try_get("imperial_date")?,
                    amount_cr: r.try_get("amount_cr")?,
                    balance_after_cr: r.try_get("balance_after_cr")?,
                    category: r.try_get("category")?,
                    description: r.try_get("description")?,
                    counterparty: r.try_get("counterparty")?,
                    created_at: r.try_get("created_at")?,
                })
            })
            .collect()
    }

    // ── Messages, Turn Transcript Replay & Hierarchical Summaries ──────────

    pub async fn list_messages(&self, campaign_id: Uuid) -> Result<Vec<MessageRecord>> {
        let rows = sqlx::query(
            "SELECT id, campaign_id, role, content, content_hash, superseded, model,
                    input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                    cost_usd, created_at
             FROM messages WHERE campaign_id = $1 ORDER BY id ASC",
        )
        .bind(campaign_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| {
                Ok(MessageRecord {
                    id: r.try_get("id")?,
                    campaign_id: r.try_get("campaign_id")?,
                    role: r.try_get("role")?,
                    content: r.try_get("content")?,
                    content_hash: r.try_get("content_hash")?,
                    superseded: r.try_get("superseded")?,
                    model: r.try_get("model")?,
                    input_tokens: r.try_get("input_tokens")?,
                    output_tokens: r.try_get("output_tokens")?,
                    cache_read_tokens: r.try_get("cache_read_tokens")?,
                    cache_creation_tokens: r.try_get("cache_creation_tokens")?,
                    cost_usd: r.try_get("cost_usd")?,
                    created_at: r.try_get("created_at")?,
                })
            })
            .collect()
    }

    pub async fn insert_message(
        &self,
        campaign_id: Uuid,
        role: &str,
        content: &str,
        content_hash: &str,
        model: Option<&str>,
        input_tokens: i32,
        output_tokens: i32,
        cache_read_tokens: i32,
        cache_creation_tokens: i32,
        cost_usd: f64,
    ) -> Result<MessageRecord> {
        let row = sqlx::query(
            "INSERT INTO messages
                (campaign_id, role, content, content_hash, model,
                 input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens, cost_usd)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
             RETURNING id, created_at",
        )
        .bind(campaign_id)
        .bind(role)
        .bind(content)
        .bind(content_hash)
        .bind(model)
        .bind(input_tokens)
        .bind(output_tokens)
        .bind(cache_read_tokens)
        .bind(cache_creation_tokens)
        .bind(cost_usd)
        .fetch_one(&self.pool)
        .await?;

        Ok(MessageRecord {
            id: row.try_get("id")?,
            campaign_id,
            role: role.to_string(),
            content: content.to_string(),
            content_hash: content_hash.to_string(),
            superseded: false,
            model: model.map(String::from),
            input_tokens,
            output_tokens,
            cache_read_tokens,
            cache_creation_tokens,
            cost_usd,
            created_at: row.try_get("created_at")?,
        })
    }

    pub async fn supersede_last_assistant_message(
        &self,
        campaign_id: Uuid,
    ) -> Result<Option<MessageRecord>> {
        let row = sqlx::query(
            "UPDATE messages SET superseded = TRUE
             WHERE id = (
                SELECT id FROM messages
                WHERE campaign_id = $1 AND role = 'assistant' AND superseded = FALSE
                ORDER BY id DESC LIMIT 1
             )
             RETURNING id, campaign_id, role, content, content_hash, superseded, model,
                       input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                       cost_usd, created_at",
        )
        .bind(campaign_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| {
            Ok(MessageRecord {
                id: r.try_get("id")?,
                campaign_id: r.try_get("campaign_id")?,
                role: r.try_get("role")?,
                content: r.try_get("content")?,
                content_hash: r.try_get("content_hash")?,
                superseded: r.try_get("superseded")?,
                model: r.try_get("model")?,
                input_tokens: r.try_get("input_tokens")?,
                output_tokens: r.try_get("output_tokens")?,
                cache_read_tokens: r.try_get("cache_read_tokens")?,
                cache_creation_tokens: r.try_get("cache_creation_tokens")?,
                cost_usd: r.try_get("cost_usd")?,
                created_at: r.try_get("created_at")?,
            })
        })
        .transpose()
    }

    pub async fn save_turn_legs(
        &self,
        campaign_id: Uuid,
        assistant_message_id: i64,
        legs: &[(Value, Value)],
    ) -> Result<()> {
        for (idx, (assistant_blocks, tool_results)) in legs.iter().enumerate() {
            sqlx::query(
                "INSERT INTO turn_legs (campaign_id, assistant_message_id, leg_index, assistant_blocks, tool_results)
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(campaign_id)
            .bind(assistant_message_id)
            .bind(idx as i32)
            .bind(assistant_blocks)
            .bind(tool_results)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub async fn load_turn_legs_map(
        &self,
        campaign_id: Uuid,
    ) -> Result<BTreeMap<i64, Vec<TurnLegRecord>>> {
        let rows = sqlx::query(
            "SELECT id, assistant_message_id, leg_index, assistant_blocks, tool_results
             FROM turn_legs WHERE campaign_id = $1 ORDER BY assistant_message_id ASC, leg_index ASC",
        )
        .bind(campaign_id)
        .fetch_all(&self.pool)
        .await?;

        let mut map: BTreeMap<i64, Vec<TurnLegRecord>> = BTreeMap::new();
        for r in rows {
            let msg_id: i64 = r.try_get("assistant_message_id")?;
            map.entry(msg_id).or_default().push(TurnLegRecord {
                id: r.try_get("id")?,
                assistant_message_id: msg_id,
                leg_index: r.try_get("leg_index")?,
                assistant_blocks: r.try_get("assistant_blocks")?,
                tool_results: r.try_get("tool_results")?,
            });
        }
        Ok(map)
    }

    pub async fn list_summaries(&self, campaign_id: Uuid) -> Result<Vec<MessageSummaryRecord>> {
        let rows = sqlx::query(
            "SELECT id, level, block_index, from_message_id, to_message_id, summary_text, model
             FROM message_summaries WHERE campaign_id = $1 ORDER BY level DESC, block_index ASC",
        )
        .bind(campaign_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| {
                Ok(MessageSummaryRecord {
                    id: r.try_get("id")?,
                    level: r.try_get("level")?,
                    block_index: r.try_get("block_index")?,
                    from_message_id: r.try_get("from_message_id")?,
                    to_message_id: r.try_get("to_message_id")?,
                    summary_text: r.try_get("summary_text")?,
                    model: r.try_get("model")?,
                })
            })
            .collect()
    }

    pub async fn upsert_summary(
        &self,
        campaign_id: Uuid,
        level: i32,
        block_index: i32,
        from_message_id: i64,
        to_message_id: i64,
        summary_text: &str,
        model: &str,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO message_summaries
                (campaign_id, level, block_index, from_message_id, to_message_id, summary_text, model)
             VALUES ($1,$2,$3,$4,$5,$6,$7)
             ON CONFLICT (campaign_id, level, block_index)
             DO UPDATE SET from_message_id = EXCLUDED.from_message_id,
                           to_message_id = EXCLUDED.to_message_id,
                           summary_text = EXCLUDED.summary_text,
                           model = EXCLUDED.model",
        )
        .bind(campaign_id)
        .bind(level)
        .bind(block_index)
        .bind(from_message_id)
        .bind(to_message_id)
        .bind(summary_text)
        .bind(model)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn log_audit(
        &self,
        campaign_id: Uuid,
        actor: &str,
        action: &str,
        diff: Value,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO state_audit_log (campaign_id, actor, action, diff) VALUES ($1,$2,$3,$4)",
        )
        .bind(campaign_id)
        .bind(actor)
        .bind(action)
        .bind(diff)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ── Ephemeral <traveller_state> Snapshot (injected after last cache breakpoint) ──

    pub async fn render_state_snapshot(&self, campaign_id: Uuid) -> Result<String> {
        let campaign = self
            .get_campaign(campaign_id)
            .await?
            .context("Campaign not found")?;
        let chars = self.list_characters(campaign_id).await?;
        let ships = self.list_ships(campaign_id).await?;
        let accounts = self.list_accounts(campaign_id).await?;

        let trade_codes = parse_uwp(&campaign.current_system_uwp)
            .map(|u| u.trade_codes.join(" "))
            .unwrap_or_default();

        let mut out = String::new();
        out.push_str("<traveller_state>\n");
        out.push_str(&format!(
            "Imperial Date: {} (next Maintenance Period in {} days)\n",
            campaign.date.format(),
            campaign.date.days_until_next_maintenance()
        ));
        out.push_str(&format!(
            "Current System: {} [{}] {}\n",
            campaign.current_system_name, campaign.current_system_uwp, trade_codes
        ));

        out.push_str("\n## Accounts\n");
        for acc in &accounts {
            out.push_str(&format!(
                "- [{:?}] {}: Cr{} (id: {})\n",
                acc.kind, acc.name, acc.balance_cr, acc.id
            ));
        }

        out.push_str("\n## Party & Crew\n");
        for row in &chars {
            let c = &row.character;
            let skills_summary: Vec<String> = c
                .skills
                .iter()
                .map(|(k, v)| format!("{k}-{v}"))
                .collect();
            let aug_count = c.augments.len();
            out.push_str(&format!(
                "- [{:?}] {} (id: {}): UPP {}/{} | Skills: {} | Augments: {} | GenXP: {} | Salary: Cr{}/mo\n",
                c.kind,
                c.name,
                c.id,
                c.upp_current.hex_code(),
                c.upp_max.hex_code(),
                if skills_summary.is_empty() { "None".to_string() } else { skills_summary.join(", ") },
                aug_count,
                c.general_xp,
                c.salary_cr
            ));
        }

        out.push_str("\n## Ships\n");
        let total_crew_salaries: i64 = chars
            .iter()
            .filter(|r| r.character.kind == CharacterKind::Crew)
            .map(|r| r.character.salary_cr)
            .sum();
        let occupants = chars
            .iter()
            .filter(|r| matches!(r.character.kind, CharacterKind::Player | CharacterKind::Crew))
            .count() as u32;

        for ship in &ships {
            let costs = compute_running_costs(
                &ship.spec,
                ship.mortgage.as_ref(),
                occupants,
                0,
                total_crew_salaries,
            );
            let valid_tag = if ship.validation.valid { "VALID" } else { "INVALID" };
            let dock_status = if ship.is_docked {
                let days = ship
                    .docked_at_total_minutes
                    .map(|t0| (campaign.date.to_total_minutes() - t0).max(0) / 1440)
                    .unwrap_or(0);
                format!("Docked at {} Starport ({days}d stayed)", ship.docked_system_name)
            } else {
                format!("In Orbit at {}", ship.docked_system_name)
            };
            out.push_str(&format!(
                "- {} ({}, {}t, J-{}/M-{}, {}, {}, id: {}): Hull {}/{}, Fuel {:.1}t ref / {:.1}t unref (cap {:.1}t), Cargo {} lot(s) / {:.1}t cap, Sensors {:?}, Stealth DM {}, Running Cost Cr{}/4wk (Mortgage Cr{}, Maint Cr{}, LifeSupp Cr{}, Salaries Cr{}), Skipped Maint: {}, Crits: {}\n",
                ship.name,
                ship.class_name,
                ship.spec.tonnage,
                ship.spec.jump_rating,
                ship.spec.thrust,
                valid_tag,
                dock_status,
                ship.id,
                ship.hull_current,
                ship.spec.hull_points_max,
                ship.fuel_refined,
                ship.fuel_unrefined,
                ship.spec.fuel_capacity_tons,
                ship.cargo.len(),
                ship.spec.cargo_capacity_tons,
                ship.spec.sensor_grade,
                ship.spec.stealth_dm,
                costs.total_cr,
                costs.mortgage_cr,
                costs.maintenance_cr,
                costs.life_support_cr,
                costs.crew_salaries_cr,
                ship.skipped_maintenance_periods,
                ship.crits.len()
            ));
        }

        if let Some(pc) = self.get_active_combat(campaign_id, "personal").await? {
            out.push_str(&format!(
                "\n## Active Personal Combat (Round {}, id: {})\n{}\n",
                pc.round,
                pc.id,
                serde_json::to_string(&pc.state).unwrap_or_default()
            ));
        }
        if let Some(sc) = self.get_active_combat(campaign_id, "space").await? {
            out.push_str(&format!(
                "\n## Active Spacecraft Combat (Round {}, Phase {}, id: {})\n{}\n",
                sc.round,
                sc.phase,
                sc.id,
                serde_json::to_string(&sc.state).unwrap_or_default()
            ));
        }

        let events = self.list_pending_events(campaign_id).await?;
        if !events.is_empty() {
            out.push_str("\n## Upcoming Scheduled Events\n");
            for ev in events.iter().take(8) {
                out.push_str(&format!("- [{}] ({}) {}\n", ev.due_label, ev.kind, ev.title));
            }
        }

        let loaded_mods = self.list_campaign_modules(campaign_id).await?;
        if !loaded_mods.is_empty() {
            out.push_str("\n## Loaded Campaign Modules (Hindsight Lore Active)\n");
            for m in &loaded_mods {
                out.push_str(&format!(
                    "- {} [{}] ({} lore chunks in Hindsight): {}\n",
                    m.module_name, m.module_slug, m.hindsight_chunks_ingested, m.referee_brief
                ));
            }
        }

        let traffic = self
            .list_system_ships(campaign_id, &campaign.current_system_name)
            .await?;
        out.push_str(&format!(
            "\n## Ships Currently In System ({} vessel(s) in {})\n",
            traffic.len(),
            campaign.current_system_name
        ));
        if traffic.is_empty() {
            out.push_str("- No ships currently detected (call `system_traffic` with `scan_system` to sweep).\n");
        } else {
            for v in &traffic {
                out.push_str(&format!(
                    "- {} ({}, {}t, TL{}, Hull {}/{}, Armor {}, M-{}, Stealth DM {}, id: {}): Orbit: {} | Disposition: {} | Transponder: {} | Captain: {} ({}) | Status: {} | Hint: {}\n",
                    v.name,
                    v.ship_class,
                    v.tonnage,
                    v.tl,
                    v.hull_current,
                    v.hull_max,
                    v.armor,
                    v.thrust,
                    v.stealth_dm,
                    v.id,
                    v.orbit_location,
                    v.disposition,
                    if v.transponder_active { "ON" } else { "DARK" },
                    v.captain_name,
                    v.faction,
                    v.status,
                    v.cargo_hint
                ));
            }
        }

        out.push_str("</traveller_state>");
        Ok(out)
    }

    pub async fn campaign_cost_totals(&self, campaign_id: Uuid) -> Result<Value> {
        let row = sqlx::query(
            "SELECT COALESCE(SUM(cost_usd), 0)::float8 AS total_cost,
                    COALESCE(SUM(input_tokens), 0)::bigint AS input_tokens,
                    COALESCE(SUM(output_tokens), 0)::bigint AS output_tokens,
                    COALESCE(SUM(cache_read_tokens), 0)::bigint AS cache_read_tokens
             FROM messages WHERE campaign_id = $1",
        )
        .bind(campaign_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(json!({
            "total_cost_usd": row.try_get::<f64, _>("total_cost")?,
            "input_tokens": row.try_get::<i64, _>("input_tokens")?,
            "output_tokens": row.try_get::<i64, _>("output_tokens")?,
            "cache_read_tokens": row.try_get::<i64, _>("cache_read_tokens")?,
        }))
    }

    // ── Universe (Sectors & Systems) ───────────────────────────────────────

    pub async fn count_sectors(&self) -> Result<i64> {
        let c: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sectors")
            .fetch_one(&self.pool)
            .await?;
        Ok(c)
    }

    pub async fn count_items(&self) -> Result<i64> {
        let c: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM items")
            .fetch_one(&self.pool)
            .await?;
        Ok(c)
    }

    pub async fn update_campaign_system(
        &self,
        campaign_id: Uuid,
        name: &str,
        uwp: &str,
        system_id: Option<Uuid>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE campaigns
             SET current_system_name = $2, current_system_uwp = $3, current_system_id = $4, updated_at = NOW()
             WHERE id = $1",
        )
        .bind(campaign_id)
        .bind(name)
        .bind(uwp)
        .bind(system_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_sectors(&self) -> Result<Vec<SectorRecord>> {
        let rows = sqlx::query(
            "SELECT s.id, s.name, s.abbreviation, s.sector_x, s.sector_y, COUNT(y.id) AS system_count
             FROM sectors s
             LEFT JOIN systems y ON y.sector_id = s.id
             GROUP BY s.id
             ORDER BY s.name ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| {
                Ok(SectorRecord {
                    id: r.try_get("id")?,
                    name: r.try_get("name")?,
                    abbreviation: r.try_get("abbreviation")?,
                    sector_x: r.try_get("sector_x")?,
                    sector_y: r.try_get("sector_y")?,
                    system_count: r.try_get("system_count")?,
                })
            })
            .collect()
    }

    pub async fn list_sector_systems(&self, sector_id: Uuid) -> Result<Vec<SystemRecord>> {
        let rows = sqlx::query(
            "SELECT y.id, y.sector_id, s.name AS sector_name, y.hex, y.name, y.uwp,
                    TRIM(y.starport) AS starport, y.tech_level, y.law_level, y.trade_codes,
                    y.bases, y.allegiance, y.travel_zone, y.gas_giants, y.global_x, y.global_y, y.metadata
             FROM systems y
             JOIN sectors s ON s.id = y.sector_id
             WHERE y.sector_id = $1
             ORDER BY y.hex ASC",
        )
        .bind(sector_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(Self::map_system).collect()
    }

    pub async fn search_systems(
        &self,
        query: Option<&str>,
        trade_code: Option<&str>,
        limit: i64,
    ) -> Result<Vec<SystemRecord>> {
        let q_like = query
            .map(|s| format!("%{}%", s.trim().to_lowercase()))
            .unwrap_or_else(|| "%".to_string());
        let rows = sqlx::query(
            "SELECT y.id, y.sector_id, s.name AS sector_name, y.hex, y.name, y.uwp,
                    TRIM(y.starport) AS starport, y.tech_level, y.law_level, y.trade_codes,
                    y.bases, y.allegiance, y.travel_zone, y.gas_giants, y.global_x, y.global_y, y.metadata
             FROM systems y
             JOIN sectors s ON s.id = y.sector_id
             WHERE (LOWER(y.name) LIKE $1 OR LOWER(s.name) LIKE $1 OR y.hex LIKE $1)
               AND ($2::text IS NULL OR $2 = ANY(y.trade_codes))
             ORDER BY (LOWER(y.name) = LOWER(TRIM(BOTH '%' FROM $1))) DESC, y.name ASC
             LIMIT $3",
        )
        .bind(&q_like)
        .bind(trade_code.filter(|s| !s.trim().is_empty()))
        .bind(limit.clamp(1, 200))
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(Self::map_system).collect()
    }

    pub async fn get_system_by_name(&self, name: &str) -> Result<Option<SystemRecord>> {
        let row = sqlx::query(
            "SELECT y.id, y.sector_id, s.name AS sector_name, y.hex, y.name, y.uwp,
                    TRIM(y.starport) AS starport, y.tech_level, y.law_level, y.trade_codes,
                    y.bases, y.allegiance, y.travel_zone, y.gas_giants, y.global_x, y.global_y, y.metadata
             FROM systems y
             JOIN sectors s ON s.id = y.sector_id
             WHERE LOWER(y.name) = LOWER($1)
             LIMIT 1",
        )
        .bind(name.trim())
        .fetch_optional(&self.pool)
        .await?;
        row.map(Self::map_system).transpose()
    }

    pub async fn list_systems_in_box(
        &self,
        min_x: i32,
        max_x: i32,
        min_y: i32,
        max_y: i32,
    ) -> Result<Vec<SystemRecord>> {
        let rows = sqlx::query(
            "SELECT y.id, y.sector_id, s.name AS sector_name, y.hex, y.name, y.uwp,
                    TRIM(y.starport) AS starport, y.tech_level, y.law_level, y.trade_codes,
                    y.bases, y.allegiance, y.travel_zone, y.gas_giants, y.global_x, y.global_y, y.metadata
             FROM systems y
             JOIN sectors s ON s.id = y.sector_id
             WHERE y.global_x BETWEEN $1 AND $2 AND y.global_y BETWEEN $3 AND $4",
        )
        .bind(min_x)
        .bind(max_x)
        .bind(min_y)
        .bind(max_y)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(Self::map_system).collect()
    }

    fn map_system(r: sqlx::postgres::PgRow) -> Result<SystemRecord> {
        Ok(SystemRecord {
            id: r.try_get("id")?,
            sector_id: r.try_get("sector_id")?,
            sector_name: r.try_get("sector_name")?,
            hex: r.try_get("hex")?,
            name: r.try_get("name")?,
            uwp: r.try_get("uwp")?,
            starport: r.try_get("starport")?,
            tech_level: r.try_get("tech_level")?,
            law_level: r.try_get("law_level")?,
            trade_codes: r.try_get("trade_codes")?,
            bases: r.try_get("bases")?,
            allegiance: r.try_get("allegiance")?,
            travel_zone: r.try_get("travel_zone")?,
            gas_giants: r.try_get("gas_giants")?,
            global_x: r.try_get("global_x")?,
            global_y: r.try_get("global_y")?,
            metadata: r.try_get("metadata")?,
        })
    }

    // ── Items Catalog ──────────────────────────────────────────────────────

    pub async fn search_items(
        &self,
        query: Option<&str>,
        category: Option<&str>,
        max_tl: Option<i16>,
        limit: i64,
    ) -> Result<Vec<ItemRecord>> {
        let q_like = query
            .map(|s| format!("%{}%", s.trim().to_lowercase()))
            .unwrap_or_else(|| "%".to_string());
        let rows = sqlx::query(
            "SELECT id, pack_id, category, name, tl, cost_cr, mass_kg, law_restriction, stats, tags
             FROM items
             WHERE LOWER(name) LIKE $1
               AND ($2::text IS NULL OR LOWER(category) = LOWER($2))
               AND ($3::int2 IS NULL OR tl <= $3)
             ORDER BY tl ASC, name ASC
             LIMIT $4",
        )
        .bind(&q_like)
        .bind(category.filter(|s| !s.trim().is_empty()))
        .bind(max_tl)
        .bind(limit.clamp(1, 200))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| {
                Ok(ItemRecord {
                    id: r.try_get("id")?,
                    pack_id: r.try_get("pack_id")?,
                    category: r.try_get("category")?,
                    name: r.try_get("name")?,
                    tl: r.try_get("tl")?,
                    cost_cr: r.try_get("cost_cr")?,
                    mass_kg: r.try_get("mass_kg")?,
                    law_restriction: r.try_get("law_restriction")?,
                    stats: r.try_get("stats")?,
                    tags: r.try_get("tags")?,
                })
            })
            .collect()
    }

    pub async fn get_item(&self, item_id: Uuid) -> Result<Option<ItemRecord>> {
        let row = sqlx::query(
            "SELECT id, pack_id, category, name, tl, cost_cr, mass_kg, law_restriction, stats, tags
             FROM items WHERE id = $1",
        )
        .bind(item_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| {
            Ok(ItemRecord {
                id: r.try_get("id")?,
                pack_id: r.try_get("pack_id")?,
                category: r.try_get("category")?,
                name: r.try_get("name")?,
                tl: r.try_get("tl")?,
                cost_cr: r.try_get("cost_cr")?,
                mass_kg: r.try_get("mass_kg")?,
                law_restriction: r.try_get("law_restriction")?,
                stats: r.try_get("stats")?,
                tags: r.try_get("tags")?,
            })
        })
        .transpose()
    }

    // ── Combats (Personal & Space) ─────────────────────────────────────────

    pub async fn get_active_combat(
        &self,
        campaign_id: Uuid,
        kind: &str,
    ) -> Result<Option<CombatRecord>> {
        let row = sqlx::query(
            "SELECT id, campaign_id, kind, status, round, phase, state
             FROM combats WHERE campaign_id = $1 AND kind = $2 AND status = 'active'
             ORDER BY updated_at DESC LIMIT 1",
        )
        .bind(campaign_id)
        .bind(kind)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| {
            Ok(CombatRecord {
                id: r.try_get("id")?,
                campaign_id: r.try_get("campaign_id")?,
                kind: r.try_get("kind")?,
                status: r.try_get("status")?,
                round: r.try_get("round")?,
                phase: r.try_get("phase")?,
                state: r.try_get("state")?,
            })
        })
        .transpose()
    }

    pub async fn upsert_combat(
        &self,
        id: Uuid,
        campaign_id: Uuid,
        kind: &str,
        status: &str,
        round: i32,
        phase: &str,
        state: &Value,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO combats (id, campaign_id, kind, status, round, phase, state)
             VALUES ($1,$2,$3,$4,$5,$6,$7)
             ON CONFLICT (id) DO UPDATE SET
                status = EXCLUDED.status, round = EXCLUDED.round, phase = EXCLUDED.phase,
                state = EXCLUDED.state, updated_at = NOW()",
        )
        .bind(id)
        .bind(campaign_id)
        .bind(kind)
        .bind(status)
        .bind(round)
        .bind(phase)
        .bind(state)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ── Scheduled Events ───────────────────────────────────────────────────

    pub async fn create_event(
        &self,
        campaign_id: Uuid,
        due_total_minutes: i64,
        due_label: &str,
        kind: &str,
        title: &str,
        payload: Value,
    ) -> Result<ScheduledEventRecord> {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO scheduled_events (id, campaign_id, due_total_minutes, due_label, kind, title, payload)
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(id)
        .bind(campaign_id)
        .bind(due_total_minutes)
        .bind(due_label)
        .bind(kind)
        .bind(title)
        .bind(&payload)
        .execute(&self.pool)
        .await?;
        Ok(ScheduledEventRecord {
            id,
            campaign_id,
            due_total_minutes,
            due_label: due_label.to_string(),
            kind: kind.to_string(),
            title: title.to_string(),
            payload,
            completed: false,
        })
    }

    pub async fn list_pending_events(&self, campaign_id: Uuid) -> Result<Vec<ScheduledEventRecord>> {
        let rows = sqlx::query(
            "SELECT id, campaign_id, due_total_minutes, due_label, kind, title, payload, completed
             FROM scheduled_events
             WHERE campaign_id = $1 AND completed = FALSE
             ORDER BY due_total_minutes ASC",
        )
        .bind(campaign_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| {
                Ok(ScheduledEventRecord {
                    id: r.try_get("id")?,
                    campaign_id: r.try_get("campaign_id")?,
                    due_total_minutes: r.try_get("due_total_minutes")?,
                    due_label: r.try_get("due_label")?,
                    kind: r.try_get("kind")?,
                    title: r.try_get("title")?,
                    payload: r.try_get("payload")?,
                    completed: r.try_get("completed")?,
                })
            })
            .collect()
    }

    pub async fn complete_due_events(
        &self,
        campaign_id: Uuid,
        now_total_minutes: i64,
    ) -> Result<Vec<ScheduledEventRecord>> {
        let rows = sqlx::query(
            "UPDATE scheduled_events SET completed = TRUE
             WHERE campaign_id = $1 AND completed = FALSE AND due_total_minutes <= $2
             RETURNING id, campaign_id, due_total_minutes, due_label, kind, title, payload, completed",
        )
        .bind(campaign_id)
        .bind(now_total_minutes)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| {
                Ok(ScheduledEventRecord {
                    id: r.try_get("id")?,
                    campaign_id: r.try_get("campaign_id")?,
                    due_total_minutes: r.try_get("due_total_minutes")?,
                    due_label: r.try_get("due_label")?,
                    kind: r.try_get("kind")?,
                    title: r.try_get("title")?,
                    payload: r.try_get("payload")?,
                    completed: r.try_get("completed")?,
                })
            })
            .collect()
    }

    // ── Campaign Modules (Knowledge Files + NPCs/Ships/Events) ─────────────

    pub async fn list_modules(&self) -> Result<Vec<ModuleRecord>> {
        let rows = sqlx::query(
            "SELECT id, slug, name, version, description, author, starting_system_name,
                    starting_system_uwp, referee_brief, knowledge_files, npcs, ships, events, tags, source
             FROM modules ORDER BY name ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(Self::map_module).collect()
    }

    pub async fn get_module(&self, module_id: Uuid) -> Result<Option<ModuleRecord>> {
        let row = sqlx::query(
            "SELECT id, slug, name, version, description, author, starting_system_name,
                    starting_system_uwp, referee_brief, knowledge_files, npcs, ships, events, tags, source
             FROM modules WHERE id = $1",
        )
        .bind(module_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(Self::map_module).transpose()
    }

    pub async fn upsert_module(&self, input: UpsertModuleInput) -> Result<ModuleRecord> {
        let id = Uuid::new_v5(&Uuid::NAMESPACE_URL, format!("module:{}", input.slug).as_bytes());
        let row = sqlx::query(
            "INSERT INTO modules
                (id, slug, name, version, description, author, starting_system_name,
                 starting_system_uwp, referee_brief, knowledge_files, npcs, ships, events, tags, source)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
             ON CONFLICT (slug) DO UPDATE SET
                name = EXCLUDED.name,
                version = EXCLUDED.version,
                description = EXCLUDED.description,
                author = EXCLUDED.author,
                starting_system_name = EXCLUDED.starting_system_name,
                starting_system_uwp = EXCLUDED.starting_system_uwp,
                referee_brief = EXCLUDED.referee_brief,
                knowledge_files = EXCLUDED.knowledge_files,
                npcs = EXCLUDED.npcs,
                ships = EXCLUDED.ships,
                events = EXCLUDED.events,
                tags = EXCLUDED.tags,
                source = EXCLUDED.source,
                updated_at = NOW()
             RETURNING id, slug, name, version, description, author, starting_system_name,
                       starting_system_uwp, referee_brief, knowledge_files, npcs, ships, events, tags, source",
        )
        .bind(id)
        .bind(&input.slug)
        .bind(&input.name)
        .bind(&input.version)
        .bind(&input.description)
        .bind(&input.author)
        .bind(&input.starting_system_name)
        .bind(&input.starting_system_uwp)
        .bind(&input.referee_brief)
        .bind(serde_json::to_value(&input.knowledge_files)?)
        .bind(serde_json::to_value(&input.npcs)?)
        .bind(serde_json::to_value(&input.ships)?)
        .bind(serde_json::to_value(&input.events)?)
        .bind(&input.tags)
        .bind(&input.source)
        .fetch_one(&self.pool)
        .await?;
        Self::map_module(row)
    }

    pub async fn delete_module(&self, module_id: Uuid) -> Result<bool> {
        let res = sqlx::query("DELETE FROM modules WHERE id = $1")
            .bind(module_id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    fn map_module(r: sqlx::postgres::PgRow) -> Result<ModuleRecord> {
        Ok(ModuleRecord {
            id: r.try_get("id")?,
            slug: r.try_get("slug")?,
            name: r.try_get("name")?,
            version: r.try_get("version")?,
            description: r.try_get("description")?,
            author: r.try_get("author")?,
            starting_system_name: r.try_get("starting_system_name")?,
            starting_system_uwp: r.try_get("starting_system_uwp")?,
            referee_brief: r.try_get("referee_brief")?,
            knowledge_files: serde_json::from_value(r.try_get("knowledge_files")?)?,
            npcs: serde_json::from_value(r.try_get("npcs")?)?,
            ships: serde_json::from_value(r.try_get("ships")?)?,
            events: serde_json::from_value(r.try_get("events")?)?,
            tags: r.try_get("tags")?,
            source: r.try_get("source")?,
        })
    }

    pub async fn list_campaign_modules(&self, campaign_id: Uuid) -> Result<Vec<CampaignModuleRecord>> {
        let rows = sqlx::query(
            "SELECT cm.id, cm.campaign_id, cm.module_id, m.slug AS module_slug, m.name AS module_name,
                    m.referee_brief, jsonb_array_length(m.knowledge_files) AS kf_count,
                    cm.hindsight_chunks_ingested, cm.npcs_spawned, cm.ships_spawned, cm.events_spawned, cm.loaded_at
             FROM campaign_modules cm
             JOIN modules m ON m.id = cm.module_id
             WHERE cm.campaign_id = $1
             ORDER BY cm.loaded_at ASC",
        )
        .bind(campaign_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| {
                Ok(CampaignModuleRecord {
                    id: r.try_get("id")?,
                    campaign_id: r.try_get("campaign_id")?,
                    module_id: r.try_get("module_id")?,
                    module_slug: r.try_get("module_slug")?,
                    module_name: r.try_get("module_name")?,
                    referee_brief: r.try_get("referee_brief")?,
                    knowledge_file_count: r.try_get::<i32, _>("kf_count")? as usize,
                    hindsight_chunks_ingested: r.try_get("hindsight_chunks_ingested")?,
                    npcs_spawned: r.try_get("npcs_spawned")?,
                    ships_spawned: r.try_get("ships_spawned")?,
                    events_spawned: r.try_get("events_spawned")?,
                    loaded_at: r.try_get("loaded_at")?,
                })
            })
            .collect()
    }

    pub async fn record_campaign_module_load(
        &self,
        campaign_id: Uuid,
        module_id: Uuid,
        hindsight_chunks_ingested: i32,
        npcs_spawned: i32,
        ships_spawned: i32,
        events_spawned: i32,
    ) -> Result<()> {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO campaign_modules
                (id, campaign_id, module_id, hindsight_chunks_ingested, npcs_spawned, ships_spawned, events_spawned)
             VALUES ($1,$2,$3,$4,$5,$6,$7)
             ON CONFLICT (campaign_id, module_id) DO UPDATE SET
                hindsight_chunks_ingested = EXCLUDED.hindsight_chunks_ingested,
                loaded_at = NOW()",
        )
        .bind(id)
        .bind(campaign_id)
        .bind(module_id)
        .bind(hindsight_chunks_ingested)
        .bind(npcs_spawned)
        .bind(ships_spawned)
        .bind(events_spawned)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ── In-System Starship Traffic ─────────────────────────────────────────

    pub async fn replace_system_ships(
        &self,
        campaign_id: Uuid,
        system_name: &str,
        vessels: &[traveller_rules::traffic::SystemVessel],
    ) -> Result<Vec<SystemShipRecord>> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM system_ships WHERE campaign_id = $1 AND LOWER(system_name) = LOWER($2)")
            .bind(campaign_id)
            .bind(system_name)
            .execute(&mut *tx)
            .await?;

        let mut out = Vec::with_capacity(vessels.len());
        for v in vessels {
            sqlx::query(
                "INSERT INTO system_ships
                    (id, campaign_id, system_name, name, ship_class, role, tonnage, tl,
                     hull_max, hull_current, armor, thrust, jump_rating, sensor_grade,
                     stealth_dm, transponder_active, disposition, orbit_location,
                     captain_name, faction, cargo_hint)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21)",
            )
            .bind(v.id)
            .bind(campaign_id)
            .bind(system_name)
            .bind(&v.name)
            .bind(&v.ship_class)
            .bind(&v.role)
            .bind(v.tonnage)
            .bind(v.tl)
            .bind(v.hull_max)
            .bind(v.hull_current)
            .bind(v.armor)
            .bind(v.thrust)
            .bind(v.jump_rating)
            .bind(&v.sensor_grade)
            .bind(v.stealth_dm)
            .bind(v.transponder_active)
            .bind(&v.disposition)
            .bind(&v.orbit_location)
            .bind(&v.captain_name)
            .bind(&v.faction)
            .bind(&v.cargo_hint)
            .execute(&mut *tx)
            .await?;

            out.push(SystemShipRecord {
                id: v.id,
                campaign_id,
                system_name: system_name.to_string(),
                name: v.name.clone(),
                ship_class: v.ship_class.clone(),
                role: v.role.clone(),
                tonnage: v.tonnage,
                tl: v.tl,
                hull_max: v.hull_max,
                hull_current: v.hull_current,
                armor: v.armor,
                thrust: v.thrust,
                jump_rating: v.jump_rating,
                sensor_grade: v.sensor_grade.clone(),
                stealth_dm: v.stealth_dm,
                transponder_active: v.transponder_active,
                disposition: v.disposition.clone(),
                orbit_location: v.orbit_location.clone(),
                captain_name: v.captain_name.clone(),
                faction: v.faction.clone(),
                cargo_hint: v.cargo_hint.clone(),
                comms_log: vec![],
                status: "in_system".to_string(),
            });
        }
        tx.commit().await?;
        Ok(out)
    }

    pub async fn list_system_ships(
        &self,
        campaign_id: Uuid,
        system_name: &str,
    ) -> Result<Vec<SystemShipRecord>> {
        let rows = sqlx::query(
            "SELECT id, campaign_id, system_name, name, ship_class, role, tonnage, tl,
                    hull_max, hull_current, armor, thrust, jump_rating, sensor_grade,
                    stealth_dm, transponder_active, disposition, orbit_location,
                    captain_name, faction, cargo_hint, comms_log, status
             FROM system_ships
             WHERE campaign_id = $1 AND LOWER(system_name) = LOWER($2) AND status <> 'departed'
             ORDER BY created_at ASC",
        )
        .bind(campaign_id)
        .bind(system_name)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(Self::map_system_ship).collect()
    }

    pub async fn get_system_ship(
        &self,
        campaign_id: Uuid,
        vessel_id: Uuid,
    ) -> Result<Option<SystemShipRecord>> {
        let row = sqlx::query(
            "SELECT id, campaign_id, system_name, name, ship_class, role, tonnage, tl,
                    hull_max, hull_current, armor, thrust, jump_rating, sensor_grade,
                    stealth_dm, transponder_active, disposition, orbit_location,
                    captain_name, faction, cargo_hint, comms_log, status
             FROM system_ships WHERE campaign_id = $1 AND id = $2",
        )
        .bind(campaign_id)
        .bind(vessel_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(Self::map_system_ship).transpose()
    }

    pub async fn update_system_ship(
        &self,
        campaign_id: Uuid,
        vessel_id: Uuid,
        disposition: Option<&str>,
        status: Option<&str>,
        comms_entry: Option<Value>,
    ) -> Result<Option<SystemShipRecord>> {
        let Some(mut ship) = self.get_system_ship(campaign_id, vessel_id).await? else {
            return Ok(None);
        };
        if let Some(d) = disposition {
            ship.disposition = d.to_string();
        }
        if let Some(s) = status {
            ship.status = s.to_string();
        }
        if let Some(entry) = comms_entry {
            ship.comms_log.push(entry);
        }
        sqlx::query(
            "UPDATE system_ships
             SET disposition = $3, status = $4, comms_log = $5, updated_at = NOW()
             WHERE campaign_id = $1 AND id = $2",
        )
        .bind(campaign_id)
        .bind(vessel_id)
        .bind(&ship.disposition)
        .bind(&ship.status)
        .bind(serde_json::to_value(&ship.comms_log)?)
        .execute(&self.pool)
        .await?;
        Ok(Some(ship))
    }

    fn map_system_ship(r: sqlx::postgres::PgRow) -> Result<SystemShipRecord> {
        Ok(SystemShipRecord {
            id: r.try_get("id")?,
            campaign_id: r.try_get("campaign_id")?,
            system_name: r.try_get("system_name")?,
            name: r.try_get("name")?,
            ship_class: r.try_get("ship_class")?,
            role: r.try_get("role")?,
            tonnage: r.try_get("tonnage")?,
            tl: r.try_get("tl")?,
            hull_max: r.try_get("hull_max")?,
            hull_current: r.try_get("hull_current")?,
            armor: r.try_get("armor")?,
            thrust: r.try_get("thrust")?,
            jump_rating: r.try_get("jump_rating")?,
            sensor_grade: r.try_get("sensor_grade")?,
            stealth_dm: r.try_get("stealth_dm")?,
            transponder_active: r.try_get("transponder_active")?,
            disposition: r.try_get("disposition")?,
            orbit_location: r.try_get("orbit_location")?,
            captain_name: r.try_get("captain_name")?,
            faction: r.try_get("faction")?,
            cargo_hint: r.try_get("cargo_hint")?,
            comms_log: serde_json::from_value(r.try_get("comms_log")?)?,
            status: r.try_get("status")?,
        })
    }
}
