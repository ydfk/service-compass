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
                    r#"{"group_id":null,"name":"No Group","public_url":"https://example.com","preferred_url_mode":"public","docker_endpoint_id":"endpoint-1","docker_container_id":"container-1","create_monitor":true,"monitor":{"name":"Keyword","monitor_type":"http_keyword","target_url_mode":"public","keyword":"Example Domain"}}"#,
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
