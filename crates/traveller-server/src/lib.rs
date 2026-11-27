pub mod auth;
pub mod routes;

use std::path::PathBuf;

use axum::middleware;
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use traveller_db::Db;
use traveller_llm::{HindsightClient, Orchestrator};

use crate::auth::{auth_middleware, AuthState};

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub orchestrator: Orchestrator,
    pub hindsight: HindsightClient,
    pub auth: AuthState,
    pub default_referee_model: String,
    pub default_worker_model: String,
    pub packs_dir: PathBuf,
}

/// Build the full TravellerLLM HTTP router with Authentik auth middleware.
/// Exposed publicly so integration tests exercise the exact production middleware and routes.
pub fn build_app(state: AppState) -> Router {
    let auth_state = state.auth.clone();
    routes::router(state)
        .layer(middleware::from_fn_with_state(auth_state, auth_middleware))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
}
