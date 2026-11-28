use std::convert::Infallible;
use std::path::PathBuf;

use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use futures_util::Stream;
use rand::rng;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use traveller_db::{NewCharacterInput, ShipRecord, UpsertModuleInput};
use traveller_llm::tools::execute_tool;
use traveller_llm::TurnEvent;
use traveller_rules::character::{Character, CharacterKind, Characteristic};
use traveller_rules::chargen::{
    finalize_lifepath_character, muster_out, run_lifepath_term, start_lifepath, Career,
    LifepathSession,
};
use traveller_rules::dice::{resolve_task, Difficulty, ModifierItem, RollMode};
use traveller_rules::finance::AccountKind;
use traveller_rules::sensors::{resolve_sensor_detection, SensorDetectionInput};
use traveller_rules::ship::{
    calculate_refit_cost, compute_running_costs, fleet_max_jump_rating, sample_free_trader,
    validate_ship, Mortgage, ShipSpec,
};
use traveller_rules::trade::compute_fleet_max_broker;
use traveller_rules::traffic::generate_system_traffic;
use traveller_rules::universe::{build_spatial_memory_scope, parse_uwp, plan_jump_route, RouteNode};
use uuid::Uuid;

use crate::auth::{auth_me, oidc_callback, oidc_login, oidc_logout, AuthUser};
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
    let auth_state = state.auth.clone();
    let auth_routes = Router::new()
        .route("/api/auth/login", get(oidc_login))
        .route("/api/auth/callback", get(oidc_callback))
        .route("/api/auth/logout", get(oidc_logout))
        .with_state(auth_state);

    Router::new()
        .route("/api/health", get(health))
        .route("/api/auth/me", get(auth_me))
        .route("/api/campaigns", get(list_campaigns).post(create_campaign))
        .route("/api/campaigns/{id}", delete(delete_campaign))
        .route("/api/campaigns/{id}/snapshot", get(get_campaign_snapshot))
        .route(
            "/api/campaigns/{id}/characters",
            get(list_characters).post(create_character),
        )
        .route(
            "/api/campaigns/{id}/characters/{char_id}",
            patch(update_character).delete(delete_character),
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
            "/api/campaigns/{id}/ships/{ship_id}/refit-quote",
            post(refit_quote_handler),
        )
        .route(
            "/api/campaigns/{id}/ships/{ship_id}/refuel",
            post(refuel_ship_handler),
        )
        .route(
            "/api/campaigns/{id}/ships/{ship_id}/dock",
            post(dock_ship_handler),
        )
        .route(
            "/api/campaigns/{id}/finances/pay-maintenance",
            post(pay_maintenance_handler),
        )
        .route(
            "/api/campaigns/{id}/accounts",
            get(list_accounts).post(create_account),
        )
        .route("/api/campaigns/{id}/accounts/transfer", post(transfer_funds))
        .route(
            "/api/campaigns/{id}/transactions",
            get(list_transactions).post(create_transaction),
        )
        .route("/api/campaigns/{id}/time/advance", post(advance_time))
        .route(
            "/api/campaigns/{id}/events",
            get(list_events).post(create_event),
        )
        .route("/api/campaigns/{id}/dice/task", post(roll_task_ui))
        .route("/api/campaigns/{id}/sensors/check", post(sensor_check_ui))
        // Universe / Star Map / Jump Routing
        .route("/api/universe/sectors", get(list_sectors))
        .route("/api/universe/sectors/{sector_id}/systems", get(list_sector_systems))
        .route("/api/universe/systems/search", get(search_systems))
        .route("/api/universe/route", post(plan_route_handler))
        .route("/api/campaigns/{id}/jump", post(execute_jump_handler))
        // Items Catalog & CSC 2023 Shop / Availability / Black Market
        .route("/api/items", get(search_items_handler))
        .route("/api/campaigns/{id}/shop/availability", post(shop_availability))
        .route("/api/campaigns/{id}/shop/purchase", post(shop_purchase))
        // Interactive Lifepath Chargen & Starport Hiring Hall
        .route("/api/chargen/start", post(chargen_start))
        .route("/api/chargen/term", post(chargen_term))
        .route("/api/chargen/muster", post(chargen_muster))
        .route("/api/campaigns/{id}/chargen/commit", post(chargen_commit))
        .route("/api/campaigns/{id}/crew/hiring-hall", post(crew_hiring_hall))
        .route("/api/campaigns/{id}/crew/hire", post(crew_hire))
        // Trade (Freight, Passengers, Mail, Speculative)
        .route("/api/campaigns/{id}/trade/market", post(trade_market))
        .route("/api/campaigns/{id}/trade/buy", post(trade_buy))
        .route("/api/campaigns/{id}/trade/sell", post(trade_sell))
        // Personal & High Guard Space Combat
        .route("/api/campaigns/{id}/combat/active", get(active_combats))
        .route("/api/campaigns/{id}/combat/personal", post(personal_combat_handler))
        .route("/api/campaigns/{id}/combat/space", post(space_combat_handler))
        // Campaign Modules (Knowledge Files -> Hindsight + NPCs/Ships/Events)
        .route("/api/modules", get(list_modules).post(create_module))
        .route("/api/modules/{module_id}", delete(delete_module))
        .route(
            "/api/campaigns/{id}/modules/{module_id}/load",
            post(load_module_into_campaign),
        )
        // In-System Starship Traffic & Ship Interactions
        .route("/api/campaigns/{id}/traffic", get(list_system_traffic))
        .route("/api/campaigns/{id}/traffic/scan", post(scan_system_traffic))
        .route(
            "/api/campaigns/{id}/traffic/{vessel_id}/interact",
            post(interact_system_vessel),
        )
        // Packs Reload
        .route("/api/packs/reload", post(reload_packs_handler))
        .route("/api/campaigns/{id}/messages", get(list_messages))
        .route("/api/campaigns/{id}/chat", post(chat_sse))
        .with_state(state)
        .merge(auth_routes)
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

    // Generate initial in-system starship traffic for the starting system
    let sys_rec = st.db.get_system_by_name(&camp.current_system_name).await.ok().flatten();
    let uwp = parse_uwp(&camp.current_system_uwp)
        .unwrap_or_else(|_| parse_uwp("A788899-C").expect("valid uwp"));
    let (bases, zone) = sys_rec
        .map(|s| (s.bases, s.travel_zone))
        .unwrap_or_else(|| (vec![], "Green".to_string()));
    let traffic = {
        let mut r = rng();
        generate_system_traffic(&mut r, &camp.current_system_name, &uwp, &bases, &zone)
    };
    let _ = st
        .db
        .replace_system_ships(camp.id, &camp.current_system_name, &traffic.vessels)
        .await;

    Ok(Json(json!({
        "campaign": camp,
        "in_system_ship_count": traffic.ship_count,
        "traffic_summary": traffic.summary
    })))
}

async fn delete_campaign(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let deleted = st.db.delete_campaign(id).await?;
    Ok(Json(json!({ "deleted": deleted })))
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
    let active_personal_combat = st.db.get_active_combat(id, "personal").await?;
    let active_space_combat = st.db.get_active_combat(id, "space").await?;
    let pending_events = st.db.list_pending_events(id).await?;
    let loaded_modules = st.db.list_campaign_modules(id).await?;
    let mut system_ships = st
        .db
        .list_system_ships(id, &camp.current_system_name)
        .await?;
    if system_ships.is_empty() {
        let sys_rec = st.db.get_system_by_name(&camp.current_system_name).await.ok().flatten();
        let uwp = parse_uwp(&camp.current_system_uwp)
            .unwrap_or_else(|_| parse_uwp("A788899-C").expect("valid uwp"));
        let (bases, zone) = sys_rec
            .map(|s| (s.bases, s.travel_zone))
            .unwrap_or_else(|| (vec![], "Green".to_string()));
        let traffic = {
            let mut r = rng();
            generate_system_traffic(&mut r, &camp.current_system_name, &uwp, &bases, &zone)
        };
        if let Ok(inserted) = st
            .db
            .replace_system_ships(id, &camp.current_system_name, &traffic.vessels)
            .await
        {
            system_ships = inserted;
        }
    }
    let sector_count = st.db.count_sectors().await.unwrap_or(0);
    let item_count = st.db.count_items().await.unwrap_or(0);
    let system_ship_count = system_ships.len();

    // Compute fleet-wide metrics: lowest Jump rating, max Broker/Steward (incl. robots & Neural Jacks),
    // and total 4-week Maintenance Period running costs across all fleet ships + crew salaries.
    let ship_specs: Vec<_> = ships.iter().map(|s| &s.spec).collect();
    let fleet_max_jump = fleet_max_jump_rating(&ship_specs);
    let roster_refs: Vec<(&Character, &[Value])> = characters
        .iter()
        .map(|r| (&r.character, r.equipment.as_slice()))
        .collect();
    let fleet_assets: Vec<Value> = ships.iter().flat_map(|s| s.cargo.iter().cloned()).collect();
    let fleet_broker = compute_fleet_max_broker(&roster_refs, &fleet_assets);

    let crew_salaries_cr: i64 = characters
        .iter()
        .filter(|r| r.character.kind == CharacterKind::Crew)
        .map(|r| r.character.salary_cr)
        .sum();
    let occupants = characters
        .iter()
        .filter(|r| matches!(r.character.kind, CharacterKind::Player | CharacterKind::Crew))
        .count() as u32;
    let mut total_mortgage_cr = 0i64;
    let mut total_maintenance_cr = 0i64;
    let mut total_life_support_cr = 0i64;
    for (idx, s) in ships.iter().enumerate() {
        let occ = if idx == 0 { occupants } else { 0 };
        let rc = compute_running_costs(&s.spec, s.mortgage.as_ref(), occ, 0, 0);
        total_mortgage_cr += rc.mortgage_cr;
        total_maintenance_cr += rc.maintenance_cr;
        total_life_support_cr += rc.life_support_cr;
    }
    let fleet_running_costs = json!({
        "mortgage_cr": total_mortgage_cr,
        "maintenance_cr": total_maintenance_cr,
        "life_support_cr": total_life_support_cr,
        "crew_salaries_cr": crew_salaries_cr,
        "total_cr": total_mortgage_cr + total_maintenance_cr + total_life_support_cr + crew_salaries_cr
    });

    let cur_sys_rec = st
        .db
        .get_system_by_name(&camp.current_system_name)
        .await
        .ok()
        .flatten();
    let cur_uwp = parse_uwp(&camp.current_system_uwp).ok();
    let current_system_info = json!({
        "name": camp.current_system_name,
        "uwp": camp.current_system_uwp,
        "starport": cur_uwp.as_ref().map(|u| u.starport.to_string()).unwrap_or_else(|| "A".into()),
        "tech_level": cur_uwp.as_ref().map(|u| u.tech_level).unwrap_or(12),
        "law_level": cur_uwp.as_ref().map(|u| u.law_level).unwrap_or(5),
        "trade_codes": cur_uwp.as_ref().map(|u| u.trade_codes.clone()).unwrap_or_default(),
        "gas_giants": cur_sys_rec.as_ref().map(|s| s.gas_giants).unwrap_or(1),
        "global_x": cur_sys_rec.as_ref().map(|s| s.global_x),
        "global_y": cur_sys_rec.as_ref().map(|s| s.global_y)
    });

    Ok(Json(json!({
        "campaign": camp,
        "current_system_info": current_system_info,
        "characters": characters,
        "ships": ships,
        "accounts": accounts,
        "transactions": transactions,
        "llm_costs": costs,
        "active_personal_combat": active_personal_combat,
        "active_space_combat": active_space_combat,
        "pending_events": pending_events,
        "loaded_modules": loaded_modules,
        "system_ships": system_ships,
        "system_ship_count": system_ship_count,
        "fleet_max_jump": fleet_max_jump,
        "fleet_broker": fleet_broker,
        "fleet_running_costs": fleet_running_costs,
        "pack_stats": { "sectors": sector_count, "items": item_count }
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
    Extension(user): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(input): Json<NewCharacterInput>,
) -> Result<Json<Value>, ApiError> {
    let created = st.db.create_character(id, input).await?;
    st.db
        .log_audit(
            id,
            &user.username,
            "create_character",
            json!({"name": created.character.name}),
        )
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
    Extension(user): Extension<AuthUser>,
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
            &user.username,
            "edit_character",
            json!({"id": char_id, "name": body.character.name}),
        )
        .await?;
    Ok(Json(json!({ "character": body.character, "equipment": equip })))
}

async fn delete_character(
    State(st): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Path((id, char_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>, ApiError> {
    let deleted = st.db.delete_character(id, char_id).await?;
    st.db
        .log_audit(id, &user.username, "delete_character", json!({"id": char_id}))
        .await?;
    Ok(Json(json!({ "deleted": deleted })))
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

async fn refit_quote_handler(
    State(st): State<AppState>,
    Path((id, ship_id)): Path<(Uuid, Uuid)>,
    Json(proposed): Json<ShipRecord>,
) -> Result<Json<Value>, ApiError> {
    let camp = st
        .db
        .get_campaign(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Campaign not found"))?;
    let existing = st
        .db
        .list_ships(id)
        .await?
        .into_iter()
        .find(|s| s.id == ship_id)
        .ok_or_else(|| anyhow::anyhow!("Ship not found"))?;
    let uwp = parse_uwp(&camp.current_system_uwp)
        .unwrap_or_else(|_| parse_uwp("A788899-C").expect("valid uwp"));
    let quote = calculate_refit_cost(
        &existing.spec,
        existing.hull_current,
        existing.crits.len(),
        &proposed.spec,
        proposed.hull_current,
        proposed.crits.len(),
        uwp.starport,
        uwp.tech_level,
    );
    let validation = validate_ship(&proposed.spec);
    Ok(Json(json!({ "quote": quote, "validation": validation })))
}

async fn update_ship(
    State(st): State<AppState>,
    Path((id, ship_id)): Path<(Uuid, Uuid)>,
    Json(mut ship): Json<ShipRecord>,
) -> Result<Json<Value>, ApiError> {
    let camp = st
        .db
        .get_campaign(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Campaign not found"))?;
    let existing = st
        .db
        .list_ships(id)
        .await?
        .into_iter()
        .find(|s| s.id == ship_id)
        .ok_or_else(|| anyhow::anyhow!("Ship not found"))?;

    // Preserve authoritative fields that cannot be freely edited via refit
    ship.campaign_id = id;
    ship.id = ship_id;
    ship.fuel_refined = existing.fuel_refined;
    ship.fuel_unrefined = existing.fuel_unrefined;
    ship.cargo = existing.cargo.clone();
    ship.mortgage = existing.mortgage.clone();

    let uwp = parse_uwp(&camp.current_system_uwp)
        .unwrap_or_else(|_| parse_uwp("A788899-C").expect("valid uwp"));
    let quote = calculate_refit_cost(
        &existing.spec,
        existing.hull_current,
        existing.crits.len(),
        &ship.spec,
        ship.hull_current,
        ship.crits.len(),
        uwp.starport,
        uwp.tech_level,
    );
    if !quote.allowed {
        return Err(anyhow::anyhow!(
            "{}",
            quote
                .block_reason
                .unwrap_or_else(|| "Refit not allowed at this starport".into())
        )
        .into());
    }
    if !quote.has_changes {
        return Err(anyhow::anyhow!("No ship refit or repair changes to apply.").into());
    }

    ship.validation = validate_ship(&ship.spec);
    if !ship.validation.valid {
        return Err(anyhow::anyhow!(
            "Proposed refit fails High Guard ship validation ({}).",
            ship.validation
                .issues
                .iter()
                .map(|i| i.message.as_str())
                .collect::<Vec<_>>()
                .join("; ")
        )
        .into());
    }

    if quote.total_cost_cr > 0 {
        let accounts = st.db.list_accounts(id).await?;
        let company = accounts
            .iter()
            .find(|a| a.kind == AccountKind::Company)
            .or_else(|| accounts.first())
            .ok_or_else(|| anyhow::anyhow!("Company account not found"))?;
        if company.balance_cr + company.credit_limit_cr < quote.total_cost_cr {
            return Err(anyhow::anyhow!(
                "Insufficient funds for shipyard refit: need Cr{}, Company account only has Cr{}.",
                quote.total_cost_cr,
                company.balance_cr
            )
            .into());
        }
        let summary = quote
            .items
            .iter()
            .map(|i| i.item.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        st.db
            .post_transaction(
                id,
                company.id,
                &camp.date.format_date_only(),
                -quote.total_cost_cr,
                "shipyard_refit",
                &format!("Shipyard refit for {}: {summary}", ship.name),
                &format!("{} Shipyard", camp.current_system_name),
            )
            .await?;
    }

    st.db.save_ship(&ship).await?;
    Ok(Json(json!({ "ship": ship, "quote": quote })))
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

// ── Manual Transactions & Scheduled Events ────────────────────────────────

async fn create_transaction(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let res = execute_tool(&st.db, &st.hindsight, id, "record_transaction", &body).await?;
    let parsed: Value = serde_json::from_str(&res.output_json)?;
    Ok(Json(parsed))
}

async fn list_events(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let events = st.db.list_pending_events(id).await?;
    Ok(Json(json!({ "events": events })))
}

async fn create_event(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let res = execute_tool(&st.db, &st.hindsight, id, "schedule_event", &body).await?;
    let parsed: Value = serde_json::from_str(&res.output_json)?;
    Ok(Json(parsed))
}

// ── Universe / Star Map / Jump Routing ────────────────────────────────────

async fn list_sectors(State(st): State<AppState>) -> Result<Json<Value>, ApiError> {
    let sectors = st.db.list_sectors().await?;
    Ok(Json(json!({ "sectors": sectors })))
}

async fn list_sector_systems(
    State(st): State<AppState>,
    Path(sector_id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let systems = st.db.list_sector_systems(sector_id).await?;
    Ok(Json(json!({ "systems": systems })))
}

#[derive(Deserialize)]
struct SearchSystemsQuery {
    q: Option<String>,
    trade_code: Option<String>,
    limit: Option<i64>,
}

async fn search_systems(
    State(st): State<AppState>,
    Query(q): Query<SearchSystemsQuery>,
) -> Result<Json<Value>, ApiError> {
    let systems = st
        .db
        .search_systems(q.q.as_deref(), q.trade_code.as_deref(), q.limit.unwrap_or(40))
        .await?;
    Ok(Json(json!({ "systems": systems })))
}

#[derive(Deserialize)]
struct PlanRouteBody {
    from_system: String,
    to_system: String,
    #[serde(default = "default_jump")]
    jump_rating: u8,
    #[serde(default = "default_true")]
    avoid_red_zones: bool,
    #[serde(default = "default_true")]
    require_refuel: bool,
}

fn default_jump() -> u8 {
    1
}

async fn plan_route_handler(
    State(st): State<AppState>,
    Json(body): Json<PlanRouteBody>,
) -> Result<Json<Value>, ApiError> {
    let start = st
        .db
        .get_system_by_name(&body.from_system)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Origin system '{}' not found", body.from_system))?;
    let goal = st
        .db
        .get_system_by_name(&body.to_system)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Destination system '{}' not found", body.to_system))?;

    let pad = i32::from(body.jump_rating.max(1)) * 4 + 8;
    let min_x = start.global_x.min(goal.global_x) - pad;
    let max_x = start.global_x.max(goal.global_x) + pad;
    let min_y = start.global_y.min(goal.global_y) - pad;
    let max_y = start.global_y.max(goal.global_y) + pad;
    let box_systems = st.db.list_systems_in_box(min_x, max_x, min_y, max_y).await?;
    let nodes: Vec<RouteNode> = box_systems
        .into_iter()
        .map(|s| RouteNode {
            id: s.id,
            name: s.name,
            hex: s.hex,
            uwp: s.uwp,
            starport: s.starport.chars().next().unwrap_or('X'),
            gas_giants: s.gas_giants,
            travel_zone: s.travel_zone,
            global_x: s.global_x,
            global_y: s.global_y,
        })
        .collect();

    let route = plan_jump_route(
        &nodes,
        start.id,
        goal.id,
        body.jump_rating,
        body.avoid_red_zones,
        body.require_refuel,
    );
    Ok(Json(json!({
        "found": route.is_some(),
        "jump_rating": body.jump_rating,
        "route": route
    })))
}

async fn execute_jump_handler(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
    Json(mut body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    if let Some(obj) = body.as_object_mut() {
        obj.insert("action".into(), json!("jump_to"));
    }
    let res = execute_tool(&st.db, &st.hindsight, id, "explore_universe", &body).await?;
    let parsed: Value = serde_json::from_str(&res.output_json)?;
    Ok(Json(parsed))
}

// ── Items Catalog & CSC 2023 Shop / Availability / Black Market ───────────

#[derive(Deserialize)]
struct SearchItemsQuery {
    q: Option<String>,
    category: Option<String>,
    max_tl: Option<i16>,
    limit: Option<i64>,
}

async fn search_items_handler(
    State(st): State<AppState>,
    Query(q): Query<SearchItemsQuery>,
) -> Result<Json<Value>, ApiError> {
    let items = st
        .db
        .search_items(
            q.q.as_deref(),
            q.category.as_deref(),
            q.max_tl,
            q.limit.unwrap_or(60),
        )
        .await?;
    Ok(Json(json!({ "items": items })))
}

async fn shop_availability(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
    Json(mut body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    if let Some(obj) = body.as_object_mut() {
        obj.insert("action".into(), json!("check_availability"));
    }
    let res = execute_tool(&st.db, &st.hindsight, id, "shop_market", &body).await?;
    let parsed: Value = serde_json::from_str(&res.output_json)?;
    Ok(Json(parsed))
}

async fn shop_purchase(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
    Json(mut body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    if let Some(obj) = body.as_object_mut() {
        obj.insert("action".into(), json!("purchase"));
    }
    let res = execute_tool(&st.db, &st.hindsight, id, "shop_market", &body).await?;
    let parsed: Value = serde_json::from_str(&res.output_json)?;
    Ok(Json(parsed))
}

// ── Interactive Lifepath Character Generation & Crew Hiring Hall ──────────

#[derive(Deserialize)]
struct ChargenStartBody {
    name: String,
    #[serde(default = "default_human")]
    species: String,
    #[serde(default)]
    background_skills: Vec<String>,
}

fn default_human() -> String {
    "Human".to_string()
}

async fn chargen_start(Json(body): Json<ChargenStartBody>) -> Result<Json<Value>, ApiError> {
    let session = {
        let mut r = rng();
        start_lifepath(&mut r, body.name, body.species, body.background_skills)
    };
    Ok(Json(json!({ "session": session })))
}

#[derive(Deserialize)]
struct ChargenTermBody {
    session: LifepathSession,
    career: Career,
    #[serde(default)]
    prefer_specialist: bool,
}

async fn chargen_term(Json(mut body): Json<ChargenTermBody>) -> Result<Json<Value>, ApiError> {
    let entry = {
        let mut r = rng();
        run_lifepath_term(&mut r, &mut body.session, body.career, body.prefer_specialist)
    };
    Ok(Json(json!({ "session": body.session, "entry": entry })))
}

#[derive(Deserialize)]
struct ChargenMusterBody {
    session: LifepathSession,
    #[serde(default = "default_cash_rolls")]
    cash_rolls: u32,
}

fn default_cash_rolls() -> u32 {
    2
}

async fn chargen_muster(Json(mut body): Json<ChargenMusterBody>) -> Result<Json<Value>, ApiError> {
    {
        let mut r = rng();
        muster_out(&mut r, &mut body.session, body.cash_rolls);
    }
    Ok(Json(json!({ "session": body.session })))
}

#[derive(Deserialize)]
struct ChargenCommitBody {
    session: LifepathSession,
    #[serde(default = "default_player_kind")]
    kind: CharacterKind,
    #[serde(default)]
    salary_cr: i64,
    #[serde(default)]
    station: Option<String>,
    #[serde(default)]
    deposit_cash_account_id: Option<Uuid>,
    #[serde(default)]
    replace_existing_player: bool,
}

fn default_player_kind() -> CharacterKind {
    CharacterKind::Player
}

async fn chargen_commit(
    State(st): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(body): Json<ChargenCommitBody>,
) -> Result<Json<Value>, ApiError> {
    if body.kind == CharacterKind::Player && body.replace_existing_player {
        st.db.delete_player_characters(id).await?;
    }
    let finalized = finalize_lifepath_character(
        id,
        body.kind,
        &body.session,
        body.salary_cr,
        body.station,
    );
    let created = st
        .db
        .create_character(
            id,
            NewCharacterInput {
                kind: finalized.kind,
                name: finalized.name.clone(),
                species: finalized.species,
                age: finalized.age,
                upp: finalized.upp_max,
                skills: finalized.skills,
                augments: finalized.augments,
                salary_cr: finalized.salary_cr,
                assigned_ship_id: finalized.assigned_ship_id,
                assigned_station: finalized.assigned_station,
                notes: finalized.notes,
            },
        )
        .await?;

    if body.session.cash_cr > 0
        && let Some(acc_id) = body.deposit_cash_account_id
    {
        let camp = st
            .db
            .get_campaign(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Campaign not found"))?;
        let date_s = camp.date.format_date_only();
        let _ = st
            .db
            .post_transaction(
                id,
                acc_id,
                &date_s,
                body.session.cash_cr,
                "mustering_out",
                &format!("Mustering out cash for {}", created.character.name),
                "Imperial Treasury",
            )
            .await?;
    }

    st.db
        .log_audit(
            id,
            &user.username,
            "chargen_commit",
            json!({"name": created.character.name, "terms": body.session.terms_completed, "cash_cr": body.session.cash_cr}),
        )
        .await?;

    Ok(Json(json!({ "character": created })))
}

async fn crew_hiring_hall(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
    Json(mut body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    if let Some(obj) = body.as_object_mut() {
        obj.insert("action".into(), json!("hiring_hall"));
    }
    let res = execute_tool(&st.db, &st.hindsight, id, "hire_crew", &body).await?;
    let parsed: Value = serde_json::from_str(&res.output_json)?;
    Ok(Json(parsed))
}

#[derive(Deserialize)]
struct HireCrewBody {
    character: Option<Character>,
    name: Option<String>,
    role: Option<String>,
    salary_cr: Option<i64>,
    ship_id: Option<Uuid>,
}

async fn crew_hire(
    State(st): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(body): Json<HireCrewBody>,
) -> Result<Json<Value>, ApiError> {
    // If the UI passes the full lifepath-generated candidate Character, persist it verbatim!
    if let Some(ch) = body.character {
        let created = st
            .db
            .create_character(
                id,
                NewCharacterInput {
                    kind: CharacterKind::Crew,
                    name: ch.name.clone(),
                    species: ch.species,
                    age: ch.age,
                    upp: ch.upp_max,
                    skills: ch.skills,
                    augments: ch.augments,
                    salary_cr: body.salary_cr.unwrap_or(ch.salary_cr),
                    assigned_ship_id: body.ship_id.or(ch.assigned_ship_id),
                    assigned_station: body.role.or(ch.assigned_station),
                    notes: ch.notes,
                },
            )
            .await?;
        st.db
            .log_audit(
                id,
                &user.username,
                "hire_crew",
                json!({"name": created.character.name, "salary_cr": created.character.salary_cr}),
            )
            .await?;
        return Ok(Json(json!({ "hired": created })));
    }

    let tool_body = json!({
        "action": "hire",
        "name": body.name,
        "role": body.role,
        "salary_cr": body.salary_cr,
        "ship_id": body.ship_id,
    });
    let res = execute_tool(&st.db, &st.hindsight, id, "hire_crew", &tool_body).await?;
    let parsed: Value = serde_json::from_str(&res.output_json)?;
    Ok(Json(parsed))
}

// ── Trade (Freight, Passengers, Mail, Speculative) ────────────────────────

async fn trade_market(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
    Json(mut body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    if let Some(obj) = body.as_object_mut() {
        obj.insert("action".into(), json!("market"));
    }
    let res = execute_tool(&st.db, &st.hindsight, id, "trade_ops", &body).await?;
    let parsed: Value = serde_json::from_str(&res.output_json)?;
    Ok(Json(parsed))
}

async fn trade_buy(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
    Json(mut body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    if let Some(obj) = body.as_object_mut() {
        obj.insert("action".into(), json!("buy_cargo"));
    }
    let res = execute_tool(&st.db, &st.hindsight, id, "trade_ops", &body).await?;
    let parsed: Value = serde_json::from_str(&res.output_json)?;
    Ok(Json(parsed))
}

async fn trade_sell(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
    Json(mut body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    if let Some(obj) = body.as_object_mut() {
        obj.insert("action".into(), json!("sell_cargo"));
    }
    let res = execute_tool(&st.db, &st.hindsight, id, "trade_ops", &body).await?;
    let parsed: Value = serde_json::from_str(&res.output_json)?;
    Ok(Json(parsed))
}

// ── Personal Combat & High Guard Space Combat ─────────────────────────────

async fn active_combats(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let personal = st.db.get_active_combat(id, "personal").await?;
    let space = st.db.get_active_combat(id, "space").await?;
    Ok(Json(json!({ "personal": personal, "space": space })))
}

async fn personal_combat_handler(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let res = execute_tool(&st.db, &st.hindsight, id, "personal_combat", &body).await?;
    let parsed: Value = serde_json::from_str(&res.output_json)?;
    Ok(Json(parsed))
}

async fn space_combat_handler(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let res = execute_tool(&st.db, &st.hindsight, id, "space_combat", &body).await?;
    let parsed: Value = serde_json::from_str(&res.output_json)?;
    Ok(Json(parsed))
}

// ── Pack Reload ───────────────────────────────────────────────────────────

async fn reload_packs_handler(State(st): State<AppState>) -> Result<Json<Value>, ApiError> {
    let mut imported_systems = 0usize;
    let mut imported_items = 0usize;
    for dir in [st.packs_dir.clone(), PathBuf::from("/app/packs/starter-srd")] {
        if dir.exists() {
            let (s, i) = traveller_db::load_packs_from_dir(&st.db, &dir).await?;
            imported_systems += s;
            imported_items += i;
        }
    }
    let total_sectors = st.db.count_sectors().await.unwrap_or(0);
    let total_items = st.db.count_items().await.unwrap_or(0);
    Ok(Json(json!({
        "imported_systems": imported_systems,
        "imported_items": imported_items,
        "total_sectors": total_sectors,
        "total_items": total_items
    })))
}

// ── Campaign Modules (Knowledge Files -> Hindsight + NPCs/Ships/Events) ───

async fn list_modules(State(st): State<AppState>) -> Result<Json<Value>, ApiError> {
    let modules = st.db.list_modules().await?;
    Ok(Json(json!({ "modules": modules })))
}

async fn create_module(
    State(st): State<AppState>,
    Json(mut input): Json<UpsertModuleInput>,
) -> Result<Json<Value>, ApiError> {
    if input.slug.trim().is_empty() {
        let s: String = input
            .name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        input.slug = s.trim_matches('-').to_string();
    }
    if input.slug.is_empty() || input.name.trim().is_empty() {
        return Err(anyhow::anyhow!("Module name is required").into());
    }
    let rec = st.db.upsert_module(input).await?;
    Ok(Json(json!({ "module": rec })))
}

async fn delete_module(
    State(st): State<AppState>,
    Path(module_id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let deleted = st.db.delete_module(module_id).await?;
    Ok(Json(json!({ "deleted": deleted })))
}

#[derive(Deserialize, Default)]
struct LoadModuleBody {
    #[serde(default)]
    move_to_starting_system: bool,
}

async fn load_module_into_campaign(
    State(st): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Path((id, module_id)): Path<(Uuid, Uuid)>,
    body: Option<Json<LoadModuleBody>>,
) -> Result<Json<Value>, ApiError> {
    let opts = body.map(|b| b.0).unwrap_or_default();
    let mut camp = st
        .db
        .get_campaign(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Campaign not found"))?;
    let module = st
        .db
        .get_module(module_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Module not found"))?;

    // 1. Ensure Hindsight bank exists for this campaign and ingest all knowledge files
    //    tagged with the module's starting system / sector / polity / government so regional recall finds them!
    let _ = st
        .hindsight
        .ensure_bank(&camp.hindsight_bank_id, &camp.name)
        .await;
    let mod_sys_name = module
        .starting_system_name
        .as_deref()
        .unwrap_or(&camp.current_system_name);
    let mod_sys_rec = st.db.get_system_by_name(mod_sys_name).await.ok().flatten();
    let mod_uwp_s = module
        .starting_system_uwp
        .as_deref()
        .or_else(|| mod_sys_rec.as_ref().map(|s| s.uwp.as_str()))
        .unwrap_or(&camp.current_system_uwp);
    let mod_uwp = parse_uwp(mod_uwp_s)
        .unwrap_or_else(|_| parse_uwp("A788899-C").expect("valid uwp"));
    let mod_scope = build_spatial_memory_scope(
        mod_sys_name,
        mod_sys_rec.as_ref().map(|s| s.sector_name.as_str()),
        &mod_uwp,
        mod_sys_rec.as_ref().map(|s| s.allegiance.as_str()).unwrap_or("Im"),
    );

    let mut chunks_ingested = 0usize;
    for kf in &module.knowledge_files {
        let title = if kf.title.trim().is_empty() {
            &kf.filename
        } else {
            &kf.title
        };
        let n = st
            .hindsight
            .retain_knowledge_document(
                &camp.hindsight_bank_id,
                &module.slug,
                &kf.filename,
                title,
                &kf.content,
                mod_scope.all_tags.clone(),
                mod_scope.entities.clone(),
            )
            .await
            .unwrap_or(0);
        chunks_ingested += n;
    }

    // 2. Spawn module NPCs (deduplicating by name, strictly never spawning a second 'player')
    let existing_chars = st.db.list_characters(id).await?;
    let existing_names: std::collections::HashSet<String> = existing_chars
        .iter()
        .map(|c| c.character.name.to_lowercase())
        .collect();
    let mut npcs_spawned = 0i32;
    for mut npc in module.npcs.clone() {
        if existing_names.contains(&npc.name.to_lowercase()) {
            continue;
        }
        if npc.kind == CharacterKind::Player {
            npc.kind = CharacterKind::Npc;
        }
        st.db.create_character(id, npc).await?;
        npcs_spawned += 1;
    }

    // 3. Spawn optional module scheduled events
    let mut events_spawned = 0i32;
    for ev in &module.events {
        let delta_mins = ev.days_from_now * 24 * 60 + ev.hours_from_now * 60;
        let due_date = camp.date.advance_minutes(delta_mins.max(60));
        let due_total = due_date.to_total_minutes();
        st.db
            .create_event(
                id,
                due_total,
                &due_date.format(),
                &ev.kind,
                &ev.title,
                json!({ "module": module.slug }),
            )
            .await?;
        events_spawned += 1;
    }

    // 4. Optional starting system move
    if opts.move_to_starting_system
        && let Some(sys_name) = &module.starting_system_name
    {
        let sys_rec = st.db.get_system_by_name(sys_name).await.ok().flatten();
        let uwp_s = module
            .starting_system_uwp
            .clone()
            .or_else(|| sys_rec.as_ref().map(|s| s.uwp.clone()))
            .unwrap_or_else(|| camp.current_system_uwp.clone());
        st.db
            .update_campaign_system(id, sys_name, &uwp_s, sys_rec.as_ref().map(|s| s.id))
            .await?;
    }

    // 5. Append module Referee brief into campaign story_ledger.threads if not already present
    if !module.referee_brief.trim().is_empty()
        && !camp.story_ledger.threads.contains(&module.name)
    {
        if !camp.story_ledger.threads.is_empty() {
            camp.story_ledger.threads.push('\n');
        }
        camp.story_ledger
            .threads
            .push_str(&format!("- [Module: {}] {}", module.name, module.referee_brief.trim()));
        st.db.update_story_ledger(id, &camp.story_ledger).await?;
    }

    st.db
        .record_campaign_module_load(
            id,
            module.id,
            chunks_ingested as i32,
            npcs_spawned,
            0,
            events_spawned,
        )
        .await?;

    st.db
        .log_audit(
            id,
            &user.username,
            "load_module",
            json!({
                "module_slug": module.slug,
                "hindsight_chunks": chunks_ingested,
                "npcs_spawned": npcs_spawned,
                "events_spawned": events_spawned
            }),
        )
        .await?;

    Ok(Json(json!({
        "loaded": true,
        "module_name": module.name,
        "hindsight_bank_id": camp.hindsight_bank_id,
        "hindsight_chunks_ingested": chunks_ingested,
        "npcs_spawned": npcs_spawned,
        "events_spawned": events_spawned
    })))
}

// ── In-System Starship Traffic & Ship Interactions ────────────────────────

async fn list_system_traffic(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let res = execute_tool(
        &st.db,
        &st.hindsight,
        id,
        "system_traffic",
        &json!({"action": "list"}),
    )
    .await?;
    let parsed: Value = serde_json::from_str(&res.output_json)?;
    Ok(Json(parsed))
}

async fn scan_system_traffic(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let res = execute_tool(
        &st.db,
        &st.hindsight,
        id,
        "system_traffic",
        &json!({"action": "scan_system"}),
    )
    .await?;
    let parsed: Value = serde_json::from_str(&res.output_json)?;
    Ok(Json(parsed))
}

async fn interact_system_vessel(
    State(st): State<AppState>,
    Path((id, vessel_id)): Path<(Uuid, Uuid)>,
    Json(mut body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    if let Some(obj) = body.as_object_mut() {
        obj.insert("vessel_id".into(), json!(vessel_id.to_string()));
    }
    let res = execute_tool(&st.db, &st.hindsight, id, "system_traffic", &body).await?;
    let parsed: Value = serde_json::from_str(&res.output_json)?;
    Ok(Json(parsed))
}

// ── Port Ops (Refuel, Docking Fees) & Manual Maintenance Payment ──────────

async fn refuel_ship_handler(
    State(st): State<AppState>,
    Path((id, ship_id)): Path<(Uuid, Uuid)>,
    Json(mut body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    if let Some(obj) = body.as_object_mut() {
        obj.insert("action".into(), json!("refuel"));
        obj.insert("ship_id".into(), json!(ship_id.to_string()));
    }
    let res = execute_tool(&st.db, &st.hindsight, id, "port_ops", &body).await?;
    let parsed: Value = serde_json::from_str(&res.output_json)?;
    Ok(Json(parsed))
}

async fn dock_ship_handler(
    State(st): State<AppState>,
    Path((id, ship_id)): Path<(Uuid, Uuid)>,
    body: Option<Json<Value>>,
) -> Result<Json<Value>, ApiError> {
    let mut payload = body.map(|b| b.0).unwrap_or_else(|| json!({}));
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("action".into(), json!("dock"));
        obj.insert("ship_id".into(), json!(ship_id.to_string()));
    }
    let res = execute_tool(&st.db, &st.hindsight, id, "port_ops", &payload).await?;
    let parsed: Value = serde_json::from_str(&res.output_json)?;
    Ok(Json(parsed))
}

async fn pay_maintenance_handler(
    State(st): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let camp = st
        .db
        .get_campaign(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Campaign not found"))?;
    let chars = st.db.list_characters(id).await?;
    let mut ships = st.db.list_ships(id).await?;
    let accounts = st.db.list_accounts(id).await?;
    let company = accounts
        .iter()
        .find(|a| a.kind == AccountKind::Company)
        .or_else(|| accounts.first())
        .ok_or_else(|| anyhow::anyhow!("No account found"))?;

    let crew_salaries_cr: i64 = chars
        .iter()
        .filter(|r| r.character.kind == CharacterKind::Crew)
        .map(|r| r.character.salary_cr)
        .sum();
    let occupants = chars
        .iter()
        .filter(|r| matches!(r.character.kind, CharacterKind::Player | CharacterKind::Crew))
        .count() as u32;

    let mut total_paid = 0i64;
    let mut ship_lines = Vec::new();
    for (idx, ship) in ships.iter_mut().enumerate() {
        let occ = if idx == 0 { occupants } else { 0 };
        let sal = if idx == 0 { crew_salaries_cr } else { 0 };
        let rc = compute_running_costs(&ship.spec, ship.mortgage.as_ref(), occ, 0, sal);
        if rc.total_cr > 0 {
            st.db
                .post_transaction(
                    id,
                    company.id,
                    &camp.date.format_date_only(),
                    -rc.total_cr,
                    "maintenance_period",
                    &format!(
                        "4-wk Maintenance Period for {} (Mortgage Cr{}, Maint Cr{}, LifeSupp Cr{}, Salaries Cr{})",
                        ship.name, rc.mortgage_cr, rc.maintenance_cr, rc.life_support_cr, rc.crew_salaries_cr
                    ),
                    "Imperial Bank / Crew Payroll",
                )
                .await?;
            total_paid += rc.total_cr;
        }
        if let Some(m) = &mut ship.mortgage
            && m.active
            && m.periods_paid < m.periods_total
        {
            m.periods_paid += 1;
        }
        ship.skipped_maintenance_periods = 0;
        st.db.save_ship(ship).await?;
        ship_lines.push(json!({ "ship": ship.name, "paid_cr": rc.total_cr, "breakdown": rc }));
    }

    Ok(Json(json!({
        "total_paid_cr": total_paid,
        "ships": ship_lines
    })))
}
