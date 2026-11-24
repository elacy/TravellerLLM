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

    pub async fn save_character(&self, c: &Character, equipment: &[Value]) -> Result<()> {
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
                    mortgage, skipped_maintenance_periods, last_annual_overhaul_year, docked_system_name
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
                 hull_current, fuel_refined, mortgage, docked_system_name)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
             RETURNING id, campaign_id, name, class_name, owner_account_id, spec, validation,
                       hull_current, fuel_refined, fuel_unrefined, crits, cargo, passengers,
                       mortgage, skipped_maintenance_periods, last_annual_overhaul_year, docked_system_name",
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
                 last_annual_overhaul_year = $16, docked_system_name = $17, updated_at = NOW()
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
            out.push_str(&format!(
                "- {} ({}, {}t, J-{}/M-{}, {}, id: {}): Hull {}/{}, Fuel {:.1}t ref / {:.1}t unref (cap {:.1}t), Sensors {:?}, Stealth DM {}, Running Cost Cr{}/4wk (Mortgage Cr{}, Maint Cr{}, LifeSupp Cr{}, Salaries Cr{}), Skipped Maint: {}, Crits: {}\n",
                ship.name,
                ship.class_name,
                ship.spec.tonnage,
                ship.spec.jump_rating,
                ship.spec.thrust,
                valid_tag,
                ship.id,
                ship.hull_current,
                ship.spec.hull_points_max,
                ship.fuel_refined,
                ship.fuel_unrefined,
                ship.spec.fuel_capacity_tons,
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
}
