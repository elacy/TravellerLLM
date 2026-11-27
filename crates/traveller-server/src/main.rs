use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Result;
use tower_http::services::{ServeDir, ServeFile};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use traveller_db::Db;
use traveller_llm::{HindsightClient, LiteLlmClient, LlmConfig, Orchestrator};
use traveller_server::auth::{AuthConfig, AuthMode, AuthState};
use traveller_server::{build_app, AppState};

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,traveller_server=debug,traveller_llm=debug")),
        )
        .init();

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3080);
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://traveller:traveller@localhost:5432/traveller".to_string());
    let litellm_base_url = std::env::var("LITELLM_BASE_URL")
        .unwrap_or_else(|_| "http://nas.lacy.ie:4000".to_string());
    let litellm_api_key = std::env::var("LITELLM_API_KEY").unwrap_or_default();
    let default_referee_model = std::env::var("REFEREE_MODEL")
        .unwrap_or_else(|_| "anthropic/claude-sonnet-4-5".to_string());
    let default_worker_model = std::env::var("WORKER_MODEL")
        .unwrap_or_else(|_| "deepseek/deepseek-v4.1-flash-1017".to_string());
    let summarizer_fallback_model = std::env::var("SUMMARIZER_FALLBACK_MODEL")
        .ok()
        .filter(|s| !s.trim().is_empty());
    let hindsight_base_url = std::env::var("HINDSIGHT_BASE_URL")
        .unwrap_or_else(|_| "http://nas.lacy.ie:8888".to_string());
    let hindsight_api_key = std::env::var("HINDSIGHT_API_KEY").ok();
    let hindsight_enabled = std::env::var("HINDSIGHT_ENABLED")
        .map(|v| !matches!(v.to_lowercase().as_str(), "0" | "false" | "no"))
        .unwrap_or(true);
    let frontend_dir = PathBuf::from(
        std::env::var("FRONTEND_DIR").unwrap_or_else(|_| "./frontend/dist".to_string()),
    );
    let packs_dir = PathBuf::from(
        std::env::var("DATA_PACKS_DIR").unwrap_or_else(|_| "./packs".to_string()),
    );

    let auth_config = AuthConfig::from_env();
    if auth_config.mode == AuthMode::None {
        warn!("AUTH_MODE=none: authentication is DISABLED (local development only)");
    } else {
        info!("Authentication enabled: {:?}", auth_config.mode);
    }

    info!("Connecting to PostgreSQL…");
    let db = Db::connect(&database_url).await?;
    db.migrate().await?;
    info!("PostgreSQL migrations up to date.");

    // Spawn non-blocking pack loader so /api/health comes up immediately while Charted Space
    // sectors & item catalogs import idempotently into PostgreSQL.
    let db_bg = db.clone();
    let packs_bg = packs_dir.clone();
    tokio::spawn(async move {
        let starter_dir = PathBuf::from("/app/packs/starter-srd");
        for dir in [packs_bg, starter_dir] {
            if dir.exists() {
                match traveller_db::load_packs_from_dir(&db_bg, &dir).await {
                    Ok((sys, items)) => {
                        if sys > 0 || items > 0 {
                            info!(
                                "Pack loader imported {sys} systems and {items} items from {}",
                                dir.display()
                            );
                        }
                    }
                    Err(e) => warn!("Pack loader error for {}: {e}", dir.display()),
                }
            }
        }
    });

    let litellm = LiteLlmClient::new(litellm_base_url, litellm_api_key);
    let hindsight = HindsightClient::new(hindsight_base_url, hindsight_api_key, hindsight_enabled);
    let orchestrator = Orchestrator::new(
        db.clone(),
        litellm,
        hindsight.clone(),
        LlmConfig {
            default_referee_model: default_referee_model.clone(),
            default_worker_model: default_worker_model.clone(),
            summarizer_fallback_model,
        },
    );

    let state = AppState {
        db,
        orchestrator,
        hindsight,
        auth: AuthState::new(auth_config),
        default_referee_model,
        default_worker_model,
        packs_dir,
    };

    let mut app = build_app(state);

    if frontend_dir.exists() {
        let index = frontend_dir.join("index.html");
        info!("Serving React Command Deck from {}", frontend_dir.display());
        app = app.fallback_service(
            ServeDir::new(&frontend_dir).not_found_service(ServeFile::new(index)),
        );
    }

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("TravellerLLM listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
