use std::convert::Infallible;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use futures_util::Stream;
use rand::rng;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use traveller_db::{NewCharacterInput, ShipRecord};
use traveller_llm::tools::execute_tool;
use traveller_llm::TurnEvent;
use traveller_rules::character::{Character, CharacterKind, Characteristic};
use traveller_rules::dice::{resolve_task, Difficulty, ModifierItem, RollMode};
use traveller_rules::finance::AccountKind;
use traveller_rules::sensors::{resolve_sensor_detection, SensorDetectionInput};
use traveller_rules::ship::{
    sample_free_trader, validate_ship, Mortgage, ShipSpec,
};
use uuid::Uuid;

use crate::AppState;

pub struct ApiError(anyhow::Error);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": self.0.to_string() })),
        )
            .into_response()
    }
}

impl<E: Into<anyhow::Error>> From<E> for ApiError {
    fn from(e: E) -> Self {
        Self(e.into())
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/campaigns", get(list_campaigns).post(create_campaign))
        .route("/api/campaigns/{id}/snapshot", get(get_campaign_snapshot))
        .route(
            "/api/campaigns/{id}/characters",
            get(list_characters).post(create_character),
        )
        .route(
            "/api/campaigns/{id}/characters/{char_id}",
            patch(update_character),
        )
        .route(
            "/api/campaigns/{id}/characters/{char_id}/spend-xp",
            post(spend_xp),
        )
        .route("/api/campaigns/{id}/ships", get(list_ships).post(create_ship))
        .route("/api/campaigns/{id}/ships/{ship_id}", patch(update_ship))
        .route(
            "/api/campaigns/{id}/ships/{ship_id}/validate",
            post(validate_ship_handler),
        )
        .route(
            "/api/campaigns/{id}/accounts",
            get(list_accounts).post(create_account),
        )
        .route("/api/campaigns/{id}/accounts/transfer", post(transfer_funds))
        .route("/api/campaigns/{id}/transactions", get(list_transactions))
        .route("/api/campaigns/{id}/time/advance", post(advance_time))
        .route("/api/campaigns/{id}/dice/task", post(roll_task_ui))
        .route("/api/campaigns/{id}/sensors/check", post(sensor_check_ui))
        .route("/api/campaigns/{id}/messages", get(list_messages))
        .route("/api/campaigns/{id}/chat", post(chat_sse))
        .with_state(state)
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok", "engine": "TravellerLLM", "ruleset": "MgT2e-2022-2024" }))
}

async fn list_campaigns(State(st): State<AppState>) -> Result<Json<Value>, ApiError> {
    let list = st.db.list_campaigns().await?;
    Ok(Json(json!({ "campaigns": list })))
}

#[derive(Deserialize)]
struct CreateCampaignBody {
    name: String,
    #[serde(default = "default_sys")]
    current_system_name: String,
    #[serde(default = "default_uwp")]
    current_system_uwp: String,
    referee_model: Option<String>,
    worker_model: Option<String>,
    #[serde(default = "default_true")]
    seed_starter_ship: bool,
}

fn default_sys() -> String {
    "Regina".to_string()
}
fn default_uwp() -> String {
    "A788899-C".to_string()
}
fn default_true() -> bool {
    true
}

async fn create_campaign(
    State(st): State<AppState>,
    Json(body): Json<CreateCampaignBody>,
) -> Result<Json<Value>, ApiError> {
    let ref_model = body
        .referee_model
        .unwrap_or_else(|| st.default_referee_model.clone());
    let work_model = body
        .worker_model
        .unwrap_or_else(|| st.default_worker_model.clone());

    let camp = st
        .db
        .create_campaign(
            &body.name,
            &body.current_system_name,
            &body.current_system_uwp,
            &ref_model,
            &work_model,
        )
        .await?;

    if body.seed_starter_ship {
        let accounts = st.db.list_accounts(camp.id).await?;
        let company_id = accounts
            .iter()
            .find(|a| a.kind == AccountKind::Company)
            .map(|a| a.id);
        let spec = sample_free_trader();
        let mortgage = Mortgage::calculate(spec.purchase_price_cr, 2, 0, false);
        st.db
            .create_ship(
                camp.id,
                "ISS Beowulf",
                "Type-A Free Trader",
                company_id,
                spec,
                Some(mortgage),
                &camp.current_system_name,
            )
            .await?;
    }

    Ok(Json(json!({ "campaign": camp })))
}

async fn get_campaign_snapshot(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let camp = st
        .db
        .get_campaign(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Campaign not found"))?;
    let characters = st.db.list_characters(id).await?;
    let ships = st.db.list_ships(id).await?;
    let accounts = st.db.list_accounts(id).await?;
    let transactions = st.db.list_transactions(id, 30).await?;
    let costs = st.db.campaign_cost_totals(id).await?;
    Ok(Json(json!({
        "campaign": camp,
        "characters": characters,
        "ships": ships,
        "accounts": accounts,
        "transactions": transactions,
        "llm_costs": costs
    })))
}

async fn list_characters(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let characters = st.db.list_characters(id).await?;
    Ok(Json(json!({ "characters": characters })))
}

async fn create_character(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<NewCharacterInput>,
) -> Result<Json<Value>, ApiError> {
    let created = st.db.create_character(id, input).await?;
    st.db
        .log_audit(id, "player_ui", "create_character", json!({"name": created.character.name}))
        .await?;
    Ok(Json(json!({ "character": created })))
}

#[derive(Deserialize)]
struct UpdateCharacterBody {
    character: Character,
    #[serde(default)]
    equipment: Option<Vec<Value>>,
}

async fn update_character(
    State(st): State<AppState>,
    Path((id, char_id)): Path<(Uuid, Uuid)>,
    Json(mut body): Json<UpdateCharacterBody>,
) -> Result<Json<Value>, ApiError> {
    let existing = st
        .db
        .get_character(id, char_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Character not found"))?;
    body.character.id = char_id;
    body.character.campaign_id = id;
    let equip = body.equipment.unwrap_or(existing.equipment);
    st.db.save_character(&body.character, &equip).await?;
    st.db
        .log_audit(
            id,
            "player_ui",
            "edit_character",
            json!({"id": char_id, "name": body.character.name}),
        )
        .await?;
    Ok(Json(json!({ "character": body.character, "equipment": equip })))
}

#[derive(Deserialize)]
struct SpendXpBody {
    skill: Option<String>,
    characteristic: Option<Characteristic>,
    #[serde(default = "default_true")]
    allow_general_xp: bool,
}

async fn spend_xp(
    State(st): State<AppState>,
    Path((id, char_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<SpendXpBody>,
) -> Result<Json<Value>, ApiError> {
    let mut row = st
        .db
        .get_character(id, char_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Character not found"))?;

    let upgraded = if let Some(skill) = body.skill {
        let lvl = row
            .character
            .upgrade_skill(&skill, body.allow_general_xp)?;
        json!({ "kind": "skill", "skill": skill, "new_level": lvl })
    } else if let Some(c) = body.characteristic {
        let lvl = row
            .character
            .upgrade_characteristic(c, body.allow_general_xp)?;
        json!({ "kind": "characteristic", "characteristic": c, "new_level": lvl })
    } else {
        return Err(anyhow::anyhow!("Specify either 'skill' or 'characteristic'").into());
    };

    st.db.save_character(&row.character, &row.equipment).await?;
    Ok(Json(json!({ "upgraded": upgraded, "character": row })))
}

async fn list_ships(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let ships = st.db.list_ships(id).await?;
    Ok(Json(json!({ "ships": ships })))
}

#[derive(Deserialize)]
struct CreateShipBody {
    name: String,
    class_name: String,
    owner_account_id: Option<Uuid>,
    spec: ShipSpec,
    #[serde(default)]
    ship_shares: u32,
    #[serde(default)]
    career_benefits: u32,
    #[serde(default)]
    remortgage_benefits: bool,
    #[serde(default = "default_true")]
    has_mortgage: bool,
}

async fn create_ship(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<CreateShipBody>,
) -> Result<Json<Value>, ApiError> {
    let camp = st
        .db
        .get_campaign(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Campaign not found"))?;
    let mortgage = if body.has_mortgage {
        Some(Mortgage::calculate(
            body.spec.purchase_price_cr,
            body.ship_shares,
            body.career_benefits,
            body.remortgage_benefits,
        ))
    } else {
        None
    };
    let ship = st
        .db
        .create_ship(
            id,
            &body.name,
            &body.class_name,
            body.owner_account_id,
            body.spec,
            mortgage,
            &camp.current_system_name,
        )
        .await?;
    Ok(Json(json!({ "ship": ship })))
}

async fn update_ship(
    State(st): State<AppState>,
    Path((id, ship_id)): Path<(Uuid, Uuid)>,
    Json(mut ship): Json<ShipRecord>,
) -> Result<Json<Value>, ApiError> {
    ship.campaign_id = id;
    ship.id = ship_id;
    ship.validation = validate_ship(&ship.spec);
    st.db.save_ship(&ship).await?;
    Ok(Json(json!({ "ship": ship })))
}

async fn validate_ship_handler(
    State(st): State<AppState>,
    Path((id, ship_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>, ApiError> {
    let ships = st.db.list_ships(id).await?;
    let ship = ships
        .into_iter()
        .find(|s| s.id == ship_id)
        .ok_or_else(|| anyhow::anyhow!("Ship not found"))?;
    let report = validate_ship(&ship.spec);
    Ok(Json(json!({ "validation": report })))
}

async fn list_accounts(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let accounts = st.db.list_accounts(id).await?;
    Ok(Json(json!({ "accounts": accounts })))
}

#[derive(Deserialize)]
struct CreateAccountBody {
    kind: AccountKind,
    name: String,
    owner_character_id: Option<Uuid>,
    #[serde(default)]
    balance_cr: i64,
    #[serde(default)]
    credit_limit_cr: i64,
}

async fn create_account(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<CreateAccountBody>,
) -> Result<Json<Value>, ApiError> {
    let acc = st
        .db
        .create_account(
            id,
            body.kind,
            &body.name,
            body.owner_character_id,
            body.balance_cr,
            body.credit_limit_cr,
        )
        .await?;
    Ok(Json(json!({ "account": acc })))
}

#[derive(Deserialize)]
struct TransferBody {
    from_account_id: Uuid,
    to_account_id: Uuid,
    amount_cr: i64,
    #[serde(default = "default_transfer_desc")]
    description: String,
}

fn default_transfer_desc() -> String {
    "Account transfer".to_string()
}

async fn transfer_funds(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<TransferBody>,
) -> Result<Json<Value>, ApiError> {
    if body.amount_cr <= 0 {
        return Err(anyhow::anyhow!("Transfer amount must be positive").into());
    }
    let camp = st
        .db
        .get_campaign(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Campaign not found"))?;
    let date_s = camp.date.format_date_only();
    let debit = st
        .db
        .post_transaction(
            id,
            body.from_account_id,
            &date_s,
            -body.amount_cr,
            "transfer",
            &body.description,
            &body.to_account_id.to_string(),
        )
        .await?;
    let credit = st
        .db
        .post_transaction(
            id,
            body.to_account_id,
            &date_s,
            body.amount_cr,
            "transfer",
            &body.description,
            &body.from_account_id.to_string(),
        )
        .await?;
    Ok(Json(json!({ "debit": debit, "credit": credit })))
}

async fn list_transactions(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let transactions = st.db.list_transactions(id, 100).await?;
    Ok(Json(json!({ "transactions": transactions })))
}

async fn advance_time(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let res = execute_tool(&st.db, &st.hindsight, id, "advance_time", &body).await?;
    let parsed: Value = serde_json::from_str(&res.output_json)?;
    Ok(Json(parsed))
}

#[derive(Deserialize)]
struct RollTaskUiBody {
    character_id: Uuid,
    skill: Option<String>,
    characteristic: Characteristic,
    #[serde(default = "default_diff")]
    difficulty: String,
    #[serde(default)]
    mode: RollMode,
    #[serde(default)]
    extra_dm: i32,
    #[serde(default = "default_true")]
    award_activity_xp: bool,
}

fn default_diff() -> String {
    "average".to_string()
}

async fn roll_task_ui(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<RollTaskUiBody>,
) -> Result<Json<Value>, ApiError> {
    let mut row = st
        .db
        .get_character(id, body.character_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Character not found"))?;
    let difficulty =
        Difficulty::from_str_loose(&body.difficulty).unwrap_or(Difficulty::Average);
    let mut mods = row
        .character
        .task_modifiers(body.skill.as_deref(), body.characteristic, difficulty);
    if body.extra_dm != 0 {
        mods.push(ModifierItem {
            label: "Situational DM".into(),
            value: body.extra_dm,
        });
    }
    let result = {
        let mut r = rng();
        resolve_task(&mut r, body.mode, difficulty, mods)
    };

    if body.award_activity_xp
        && let Some(sk) = &body.skill {
            row.character.award_skill_xp(sk, 1);
            st.db.save_character(&row.character, &row.equipment).await?;
        }
    Ok(Json(json!({
        "character": row.character.name,
        "kind": row.character.kind == CharacterKind::Player,
        "result": result
    })))
}

async fn sensor_check_ui(
    Path(_id): Path<Uuid>,
    Json(input): Json<SensorDetectionInput>,
) -> Result<Json<Value>, ApiError> {
    let mut r = rng();
    let res = resolve_sensor_detection(&mut r, &input);
    Ok(Json(json!({ "sensor_check": res })))
}

async fn list_messages(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let messages = st.db.list_messages(id).await?;
    Ok(Json(json!({ "messages": messages })))
}

#[derive(Deserialize)]
struct ChatBody {
    message: String,
}

async fn chat_sse(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<ChatBody>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = mpsc::channel::<TurnEvent>(64);
    let orch = st.orchestrator.clone();
    let msg = body.message;
    tokio::spawn(async move {
        if let Err(e) = orch.run_turn(id, msg, tx.clone()).await {
            let _ = tx
                .send(TurnEvent::Error {
                    message: e.to_string(),
                })
                .await;
        }
    });

    let stream = ReceiverStream::new(rx).map(|ev| {
        let data = serde_json::to_string(&ev).unwrap_or_else(|_| "{}".to_string());
        Ok(Event::default().data(data))
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}
