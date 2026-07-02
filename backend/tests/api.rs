use std::{path::Path, sync::Arc};

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use service_compass_backend::{app, config::Config, db, state::AppState};
use tower::ServiceExt;

async fn test_state() -> AppState {
    let config = Arc::new(Config {
        bind: "127.0.0.1:0".into(),
        database_url: "sqlite::memory:".into(),
        secret_key: Some("test-secret".into()),
        secret_file: Path::new("target/test-secret.key").into(),
        static_dir: Path::new("missing").into(),
        log_dir: Path::new("target/test-logs").into(),
        production: false,
    });
    AppState::new(db::test_pool().await, config).unwrap()
}

async fn test_app() -> axum::Router {
    app(test_state().await, Path::new("missing"))
}

async fn login_token(app: &axum::Router) -> String {
    let login = app
        .clone()
        .oneshot(
            Request::post("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"username":"admin","password":"admin"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login.status(), StatusCode::OK);
    let body = to_bytes(login.into_body(), 1024).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    payload["token"].as_str().unwrap().to_owned()
}

async fn login_cookie(app: &axum::Router) -> String {
    let login = app
        .clone()
        .oneshot(
            Request::post("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"username":"admin","password":"admin"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login.status(), StatusCode::OK);
    login
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}

#[tokio::test]
async fn health_is_public() {
    let response = test_app()
        .await
        .oneshot(Request::get("/api/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn groups_require_a_session() {
    let response = test_app()
        .await
        .oneshot(Request::get("/api/groups").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_creates_an_accepted_session() {
    let app = test_app().await;
    let token = login_token(&app).await;

    let response = app
        .oneshot(
            Request::get("/api/groups")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn public_content_requires_login_outside_anonymous_networks() {
    let response = test_app()
        .await
        .oneshot(
            Request::get("/api/dashboard")
                .header("x-forwarded-for", "8.8.8.8")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn logged_in_external_client_can_read_public_content() {
    let app = test_app().await;
    let token = login_token(&app).await;

    let response = app
        .oneshot(
            Request::get("/api/dashboard")
                .header("x-forwarded-for", "8.8.8.8")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn session_cookie_can_read_public_content_from_external_client() {
    let app = test_app().await;
    let cookie = login_cookie(&app).await;

    let response = app
        .oneshot(
            Request::get("/api/dashboard")
                .header("x-forwarded-for", "8.8.8.8")
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn settings_reject_invalid_anonymous_networks() {
    let app = test_app().await;
    let token = login_token(&app).await;

    let response = app
        .oneshot(
            Request::put("/api/settings")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"retention_days":30,"log_retention_days":30,"cert_expiry_warning_days":30,"notification_cooldown_sec":300,"dashboard_refresh_interval_sec":30,"anonymous_access_cidrs":"192.168.1.0/33"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn service_can_be_created_without_group_and_with_monitor() {
    let app = test_app().await;
    let token = login_token(&app).await;

    let response = app
        .clone()
        .oneshot(
            Request::post("/api/services")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"group_id":null,"name":"No Group","public_url":"https://example.com","docker_endpoint_id":"endpoint-1","docker_container_id":"container-1","create_monitor":true,"monitor":{"name":"Keyword","monitor_type":"http_keyword","target_url_mode":"public","keyword":"Example Domain"}}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let monitors = app
        .clone()
        .oneshot(
            Request::get("/api/monitors")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = to_bytes(monitors.into_body(), 4096).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let monitors = payload.as_array().unwrap();
    assert_eq!(monitors.len(), 2);
    assert!(
        monitors
            .iter()
            .any(|item| item["monitor_type"] == "http_keyword")
    );
    assert!(monitors.iter().any(|item| item["monitor_type"] == "docker"));

    let dashboard = app
        .oneshot(Request::get("/api/dashboard").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(dashboard.status(), StatusCode::OK);
    let body = to_bytes(dashboard.into_body(), 16_384).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let tracks = &payload["groups"][0]["services"][0]["monitor_tracks"];
    assert_eq!(tracks.as_array().map(Vec::len), Some(2));
}

#[tokio::test]
async fn service_can_be_created_without_urls() {
    let app = test_app().await;
    let token = login_token(&app).await;

    let response = app
        .oneshot(
            Request::post("/api/services")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"group_id":null,"name":"Only Display","icon_type":"initial","enabled":true,"sort_order":0,"create_monitor":false,"cert_expiry_notify":false}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(payload["public_url"].is_null());
    assert!(payload["local_url"].is_null());
    assert!(payload.get("preferred_url_mode").is_none());
}

#[tokio::test]
async fn postgres_monitor_can_be_created_with_secret_password() {
    let app = test_app().await;
    let token = login_token(&app).await;

    let response = app
        .oneshot(
            Request::post("/api/monitors")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"service_id":null,"name":"PostgreSQL","monitor_type":"postgres","target_url":"127.0.0.1","target_url_mode":"custom","domain":"postgres","auth_username":"postgres","auth_password":"secret","expected_value":"SELECT 1","cert_port":5432,"enabled":true}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["monitor_type"], "postgres");
    assert_eq!(payload["has_auth_password"], true);
    assert!(payload.get("auth_password").is_none());
}

#[tokio::test]
async fn postgres_monitor_rejects_write_sql() {
    let app = test_app().await;
    let token = login_token(&app).await;

    let response = app
        .oneshot(
            Request::post("/api/monitors")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"service_id":null,"name":"PostgreSQL","monitor_type":"postgres","target_url":"127.0.0.1","target_url_mode":"custom","domain":"postgres","auth_username":"postgres","expected_value":"DELETE FROM users","cert_port":5432,"enabled":true}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn notification_channel_with_bad_secret_does_not_break_list() {
    let state = test_state().await;
    sqlx::query(
        "INSERT INTO notification_channels (id, name, channel_type, enabled, config_secret, created_at, updated_at) \
         VALUES ('bad-channel', 'Bad Channel', 'webhook', true, 'old-or-broken-secret', 'now', 'now')",
    )
    .execute(&state.pool)
    .await
    .unwrap();
    let app = app(state, Path::new("missing"));
    let token = login_token(&app).await;

    let response = app
        .oneshot(
            Request::get("/api/notifications/channels")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload[0]["configured"], false);
    assert_eq!(payload[0]["config"], serde_json::json!({}));
}
