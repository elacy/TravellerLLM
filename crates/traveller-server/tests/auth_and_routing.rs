use std::collections::HashSet;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::middleware;
use axum::routing::get;
use axum::Router;
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;
use traveller_server::auth::{
    auth_me, auth_middleware, AuthConfig, AuthMode, AuthState, AuthUser, OidcConfig,
};

fn build_auth_test_router(cfg: AuthConfig) -> (Router, AuthState) {
    let state = AuthState::new(cfg);
    let app = Router::new()
        .route("/api/health", get(|| async { "ok" }))
        .route("/api/auth/me", get(auth_me))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));
    (app, state)
}

#[tokio::test]
async fn authentik_proxy_enforces_headers_group_allowlist_and_health_bypass() {
    let mut allowed_groups = HashSet::new();
    allowed_groups.insert("traveller-players".to_string());

    let cfg = AuthConfig {
        mode: AuthMode::AuthentikProxy,
        allowed_users: HashSet::new(),
        allowed_groups,
        logout_url: "/outpost.goauthentik.io/sign_out".into(),
        login_url: "https://auth.i.lacy.ie".into(),
        oidc: None,
    };
    let (app, _) = build_auth_test_router(cfg);

    // 1. /api/health always bypasses auth for Docker healthchecks
    let health_resp = app
        .clone()
        .oneshot(Request::builder().uri("/api/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(health_resp.status(), StatusCode::OK);

    // 2. Missing X-authentik-username returns 401 with Authentik login_url
    let unauth_resp = app
        .clone()
        .oneshot(Request::builder().uri("/api/auth/me").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(unauth_resp.status(), StatusCode::UNAUTHORIZED);
    let body_bytes = unauth_resp.into_body().collect().await.unwrap().to_bytes();
    let json_body: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json_body["login_url"], "https://auth.i.lacy.ie");

    // 3. Valid Authentik headers with wrong group returns 403 Forbidden
    let forbidden_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/auth/me")
                .header("X-authentik-username", "guest")
                .header("X-authentik-groups", "media-users|plex")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(forbidden_resp.status(), StatusCode::FORBIDDEN);

    // 4. Valid Authentik headers with allowed group returns 200 and full AuthUser
    let ok_resp = app
        .oneshot(
            Request::builder()
                .uri("/api/auth/me")
                .header("X-authentik-username", "eva")
                .header("X-authentik-email", "eva@lacy.ie")
                .header("X-authentik-name", "Eva")
                .header("X-authentik-uid", "ak-uid-123")
                .header("X-authentik-groups", "admins|traveller-players")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ok_resp.status(), StatusCode::OK);
    let ok_bytes = ok_resp.into_body().collect().await.unwrap().to_bytes();
    let ok_json: Value = serde_json::from_slice(&ok_bytes).unwrap();
    assert_eq!(ok_json["user"]["username"], "eva");
    assert_eq!(ok_json["user"]["email"], "eva@lacy.ie");
    assert_eq!(ok_json["user"]["auth_mode"], "authentik_proxy");
    assert_eq!(
        ok_json["user"]["logout_url"],
        "/outpost.goauthentik.io/sign_out"
    );
}

#[tokio::test]
async fn oidc_signed_session_cookie_authenticates_and_rejects_tampering() {
    let cfg = AuthConfig {
        mode: AuthMode::Oidc,
        allowed_users: HashSet::new(),
        allowed_groups: HashSet::new(),
        logout_url: "/api/auth/logout".into(),
        login_url: "/api/auth/login".into(),
        oidc: Some(OidcConfig {
            issuer: "https://auth.i.lacy.ie/application/o/travellerllm".into(),
            client_id: "traveller".into(),
            client_secret: "secret".into(),
            redirect_uri: "https://traveller.i.lacy.ie/api/auth/callback".into(),
            session_secret: "super-secret-signing-key".into(),
        }),
    };
    let (app, state) = build_auth_test_router(cfg);

    let user = AuthUser {
        username: "eva".into(),
        email: Some("eva@lacy.ie".into()),
        display_name: Some("Eva".into()),
        uid: Some("sub-42".into()),
        groups: vec!["admins".into()],
        auth_mode: AuthMode::Oidc,
        logout_url: Some("/api/auth/logout".into()),
    };
    let cookie = state.sign_session(&user).unwrap();

    // Valid signed cookie succeeds
    let ok_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/auth/me")
                .header(header::COOKIE, format!("traveller_session={cookie}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ok_resp.status(), StatusCode::OK);

    // Tampered signature rejected with 401
    let tampered = format!("{cookie}deadbeef");
    let bad_resp = app
        .oneshot(
            Request::builder()
                .uri("/api/auth/me")
                .header(header::COOKIE, format!("traveller_session={tampered}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad_resp.status(), StatusCode::UNAUTHORIZED);
}
