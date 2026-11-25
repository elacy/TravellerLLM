use std::collections::HashSet;
use std::sync::Arc;

use axum::extract::{Query, Request, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};
use axum::Json;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;

const SESSION_COOKIE_NAME: &str = "traveller_session";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMode {
    /// Traefik + Authentik Proxy Provider (ForwardAuth headers `X-authentik-*`).
    AuthentikProxy,
    /// Direct Authentik OIDC Authorization Code flow with signed session cookie.
    Oidc,
    /// Disabled (local development only).
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub username: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub uid: Option<String>,
    pub groups: Vec<String>,
    pub auth_mode: AuthMode,
    pub logout_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OidcConfig {
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub session_secret: String,
}

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub mode: AuthMode,
    pub allowed_users: HashSet<String>,
    pub allowed_groups: HashSet<String>,
    /// Optional shared secret (`X-Auth-Proxy-Secret`) presented by Traefik to prevent clients
    /// hitting the published container port directly from spoofing `X-authentik-*` headers.
    pub proxy_secret: Option<String>,
    pub logout_url: String,
    pub login_url: String,
    pub oidc: Option<OidcConfig>,
}

#[derive(Clone)]
pub struct AuthState {
    pub config: Arc<AuthConfig>,
    http: Client,
    pending_states: Arc<RwLock<HashSet<String>>>,
}

impl AuthConfig {
    pub fn from_env() -> Self {
        let mode = match std::env::var("AUTH_MODE")
            .unwrap_or_else(|_| "authentik_proxy".to_string())
            .trim()
            .to_lowercase()
            .as_str()
        {
            "none" | "disabled" | "off" => AuthMode::None,
            "oidc" => AuthMode::Oidc,
            _ => AuthMode::AuthentikProxy,
        };

        let parse_csv = |var: &str| -> HashSet<String> {
            std::env::var(var)
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect()
        };

        let logout_url = std::env::var("AUTH_LOGOUT_URL").unwrap_or_else(|_| match mode {
            AuthMode::AuthentikProxy => "/outpost.goauthentik.io/sign_out".to_string(),
            AuthMode::Oidc => "/api/auth/logout".to_string(),
            AuthMode::None => String::new(),
        });
        let login_url = std::env::var("AUTH_LOGIN_URL").unwrap_or_else(|_| match mode {
            AuthMode::AuthentikProxy => "https://auth.i.lacy.ie".to_string(),
            AuthMode::Oidc => "/api/auth/login".to_string(),
            AuthMode::None => String::new(),
        });

        let oidc = if mode == AuthMode::Oidc {
            Some(OidcConfig {
                issuer: std::env::var("OIDC_ISSUER")
                    .unwrap_or_else(|_| "https://auth.i.lacy.ie/application/o/travellerllm/".into())
                    .trim_end_matches('/')
                    .to_string(),
                client_id: std::env::var("OIDC_CLIENT_ID").unwrap_or_default(),
                client_secret: std::env::var("OIDC_CLIENT_SECRET").unwrap_or_default(),
                redirect_uri: std::env::var("OIDC_REDIRECT_URI")
                    .unwrap_or_else(|_| "https://traveller.i.lacy.ie/api/auth/callback".into()),
                session_secret: std::env::var("SESSION_SECRET")
                    .unwrap_or_else(|_| uuid::Uuid::new_v4().to_string()),
            })
        } else {
            None
        };

        let proxy_secret = std::env::var("AUTH_PROXY_SECRET")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        Self {
            mode,
            allowed_users: parse_csv("AUTH_ALLOWED_USERS"),
            allowed_groups: parse_csv("AUTH_ALLOWED_GROUPS"),
            proxy_secret,
            logout_url,
            login_url,
            oidc,
        }
    }

    pub fn is_authorized(&self, user: &AuthUser) -> bool {
        if !self.allowed_users.is_empty()
            && !self.allowed_users.contains(&user.username.to_lowercase())
        {
            return false;
        }
        if !self.allowed_groups.is_empty()
            && !user
                .groups
                .iter()
                .any(|g| self.allowed_groups.contains(&g.to_lowercase()))
        {
            return false;
        }
        true
    }
}

impl AuthState {
    pub fn new(config: AuthConfig) -> Self {
        Self {
            config: Arc::new(config),
            http: Client::new(),
            pending_states: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    pub fn sign_session(&self, user: &AuthUser) -> Option<String> {
        let secret = &self.config.oidc.as_ref()?.session_secret;
        let payload = serde_json::to_string(user).ok()?;
        let payload_hex = hex_encode(payload.as_bytes());
        let sig = hmac_sha256_hex(secret.as_bytes(), payload_hex.as_bytes());
        Some(format!("{payload_hex}.{sig}"))
    }

    pub fn verify_session(&self, token: &str) -> Option<AuthUser> {
        let secret = &self.config.oidc.as_ref()?.session_secret;
        let (payload_hex, sig) = token.split_once('.')?;
        let expected = hmac_sha256_hex(secret.as_bytes(), payload_hex.as_bytes());
        if sig != expected {
            return None;
        }
        let bytes = hex_decode(payload_hex)?;
        serde_json::from_slice(&bytes).ok()
    }
}

fn header_str(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    for part in raw.split(';') {
        let part = part.trim();
        if let Some((k, v)) = part.split_once('=')
            && k.trim() == name
        {
            return Some(v.trim().to_string());
        }
    }
    None
}

fn extract_authentik_proxy_user(headers: &HeaderMap, cfg: &AuthConfig) -> Option<AuthUser> {
    if let Some(expected) = &cfg.proxy_secret {
        let presented = header_str(headers, "x-auth-proxy-secret")?;
        if presented != *expected {
            return None;
        }
    }
    let username = header_str(headers, "x-authentik-username")?;
    let email = header_str(headers, "x-authentik-email");
    let display_name = header_str(headers, "x-authentik-name");
    let uid = header_str(headers, "x-authentik-uid");
    let groups = header_str(headers, "x-authentik-groups")
        .map(|g| {
            g.split(['|', ','])
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    Some(AuthUser {
        username,
        email,
        display_name,
        uid,
        groups,
        auth_mode: AuthMode::AuthentikProxy,
        logout_url: if cfg.logout_url.is_empty() {
            None
        } else {
            Some(cfg.logout_url.clone())
        },
    })
}

/// Axum middleware enforcing Authentik ForwardAuth / OIDC on all `/api/*` routes except
/// `/api/health` and `/api/auth/{login,callback,logout}`.
pub async fn auth_middleware(
    State(auth): State<AuthState>,
    mut req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path();
    if path == "/api/health"
        || path == "/api/auth/login"
        || path == "/api/auth/callback"
        || path == "/api/auth/logout"
        || !path.starts_with("/api/")
    {
        return next.run(req).await;
    }

    let user = match auth.config.mode {
        AuthMode::None => Some(AuthUser {
            username: "local-dev".to_string(),
            email: None,
            display_name: Some("Local Dev".to_string()),
            uid: None,
            groups: vec!["admins".to_string()],
            auth_mode: AuthMode::None,
            logout_url: None,
        }),
        AuthMode::AuthentikProxy => extract_authentik_proxy_user(req.headers(), &auth.config),
        AuthMode::Oidc => cookie_value(req.headers(), SESSION_COOKIE_NAME)
            .and_then(|tok| auth.verify_session(&tok)),
    };

    let Some(user) = user else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "Authentication required",
                "auth_mode": auth.config.mode,
                "login_url": auth.config.login_url,
            })),
        )
            .into_response();
    };

    if !auth.config.is_authorized(&user) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": format!("User '{}' is not authorized for TravellerLLM", user.username),
            })),
        )
            .into_response();
    }

    req.extensions_mut().insert(user);
    next.run(req).await
}

pub async fn auth_me(req: Request) -> Response {
    if let Some(user) = req.extensions().get::<AuthUser>() {
        Json(json!({ "user": user })).into_response()
    } else {
        (StatusCode::UNAUTHORIZED, Json(json!({"error": "Not authenticated"}))).into_response()
    }
}

pub async fn oidc_login(State(auth): State<AuthState>) -> Response {
    let Some(oidc) = &auth.config.oidc else {
        return Redirect::temporary(&auth.config.login_url).into_response();
    };
    let state_tok = uuid::Uuid::new_v4().to_string();
    auth.pending_states.write().await.insert(state_tok.clone());
    // Authentik standard OIDC authorize endpoint: <issuer>/../authorize/ or discovery
    let base = oidc.issuer.trim_end_matches('/');
    let authorize_url = format!(
        "{}/authorize/?client_id={}&redirect_uri={}&response_type=code&scope=openid%20profile%20email%20groups&state={}",
        base.rsplit_once("/o/").map(|(root, _)| format!("{root}/o")).unwrap_or_else(|| base.to_string()),
        url_encode(&oidc.client_id),
        url_encode(&oidc.redirect_uri),
        url_encode(&state_tok)
    );
    Redirect::temporary(&authorize_url).into_response()
}

#[derive(Deserialize)]
pub struct OidcCallbackQuery {
    pub code: String,
    pub state: String,
}

pub async fn oidc_callback(
    State(auth): State<AuthState>,
    Query(q): Query<OidcCallbackQuery>,
) -> Response {
    let Some(oidc) = &auth.config.oidc else {
        return (StatusCode::BAD_REQUEST, "OIDC not enabled").into_response();
    };
    if !auth.pending_states.write().await.remove(&q.state) {
        return (StatusCode::BAD_REQUEST, "Invalid OIDC state").into_response();
    }

    let base = oidc.issuer.trim_end_matches('/');
    let oauth_root = base
        .rsplit_once("/o/")
        .map(|(root, _)| format!("{root}/o"))
        .unwrap_or_else(|| base.to_string());

    let token_resp = match auth
        .http
        .post(format!("{oauth_root}/token/"))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", q.code.as_str()),
            ("redirect_uri", oidc.redirect_uri.as_str()),
            ("client_id", oidc.client_id.as_str()),
            ("client_secret", oidc.client_secret.as_str()),
        ])
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => r.json::<Value>().await.unwrap_or_default(),
        _ => return (StatusCode::BAD_GATEWAY, "OIDC token exchange failed").into_response(),
    };

    let Some(access_token) = token_resp.get("access_token").and_then(|v| v.as_str()) else {
        return (StatusCode::BAD_GATEWAY, "Missing access_token").into_response();
    };

    let info = match auth
        .http
        .get(format!("{oauth_root}/userinfo/"))
        .bearer_auth(access_token)
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => r.json::<Value>().await.unwrap_or_default(),
        _ => return (StatusCode::BAD_GATEWAY, "OIDC userinfo failed").into_response(),
    };

    let username = info
        .get("preferred_username")
        .or_else(|| info.get("email"))
        .or_else(|| info.get("sub"))
        .and_then(|v| v.as_str())
        .unwrap_or("traveller")
        .to_string();
    let groups = info
        .get("groups")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let user = AuthUser {
        username,
        email: info.get("email").and_then(|v| v.as_str()).map(String::from),
        display_name: info.get("name").and_then(|v| v.as_str()).map(String::from),
        uid: info.get("sub").and_then(|v| v.as_str()).map(String::from),
        groups,
        auth_mode: AuthMode::Oidc,
        logout_url: Some("/api/auth/logout".to_string()),
    };

    let Some(signed) = auth.sign_session(&user) else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "Session signing failed").into_response();
    };

    let cookie = format!(
        "{SESSION_COOKIE_NAME}={signed}; Path=/; HttpOnly; SameSite=Lax; Max-Age=604800"
    );
    let mut resp = Redirect::temporary("/").into_response();
    if let Ok(val) = header::HeaderValue::from_str(&cookie) {
        resp.headers_mut().insert(header::SET_COOKIE, val);
    }
    resp
}

pub async fn oidc_logout(State(auth): State<AuthState>) -> Response {
    let target = if let Some(oidc) = &auth.config.oidc {
        format!("{}/end-session/", oidc.issuer.trim_end_matches('/'))
    } else if !auth.config.logout_url.is_empty() {
        auth.config.logout_url.clone()
    } else {
        "/".to_string()
    };
    let mut resp = Redirect::temporary(&target).into_response();
    if let Ok(val) = header::HeaderValue::from_str(&format!(
        "{SESSION_COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0"
    )) {
        resp.headers_mut().insert(header::SET_COOKIE, val);
    }
    resp
}

fn hmac_sha256_hex(key: &[u8], data: &[u8]) -> String {
    let mut k_block = [0u8; 64];
    if key.len() > 64 {
        let mut h = Sha256::new();
        h.update(key);
        k_block[..32].copy_from_slice(&h.finalize());
    } else {
        k_block[..key.len()].copy_from_slice(key);
    }
    let mut i_pad = [0x36u8; 64];
    let mut o_pad = [0x5cu8; 64];
    for i in 0..64 {
        i_pad[i] ^= k_block[i];
        o_pad[i] ^= k_block[i];
    }
    let mut inner = Sha256::new();
    inner.update(i_pad);
    inner.update(data);
    let inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(o_pad);
    outer.update(inner_hash);
    hex_encode(&outer.finalize())
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

fn url_encode(s: &str) -> String {
    s.bytes()
        .flat_map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![b as char]
            }
            _ => format!("%{b:02X}").chars().collect(),
        })
        .collect()
}
