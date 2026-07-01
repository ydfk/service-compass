use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use axum::{
    Json,
    extract::{Request, State},
    http::header::{AUTHORIZATION, SET_COOKIE},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    access,
    error::{AppError, AppResult},
    state::AppState,
};

#[derive(Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
pub struct SessionResponse {
    token: String,
    username: String,
}

#[derive(Deserialize)]
pub struct CredentialsRequest {
    current_password: String,
    username: String,
    new_password: String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginRequest>,
) -> AppResult<impl IntoResponse> {
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT username, password_hash FROM admin_users WHERE id = 1")
            .fetch_optional(&state.pool)
            .await?;
    let Some((username, password_hash)) = row else {
        return Err(AppError::Unauthorized);
    };
    if input.username.trim() != username || !verify_password(&input.password, &password_hash) {
        tracing::warn!(username = input.username.trim(), "管理员登录失败");
        return Err(AppError::Unauthorized);
    }
    let token = Uuid::new_v4().to_string();
    state.sessions.write().await.insert(token.clone());
    tracing::info!(username, "管理员登录成功");
    Ok((
        [(SET_COOKIE, session_cookie(&token))],
        Json(SessionResponse { token, username }),
    ))
}

pub async fn me(State(state): State<AppState>) -> AppResult<Json<serde_json::Value>> {
    let username: String = sqlx::query_scalar("SELECT username FROM admin_users WHERE id = 1")
        .fetch_one(&state.pool)
        .await?;
    Ok(Json(
        serde_json::json!({ "authenticated": true, "username": username }),
    ))
}

pub async fn update_credentials(
    State(state): State<AppState>,
    Json(input): Json<CredentialsRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let username = input.username.trim();
    if username.is_empty() || input.new_password.len() < 6 {
        return Err(AppError::Validation(
            "用户名不能为空，新密码至少需要 6 个字符".into(),
        ));
    }
    let current_hash: String =
        sqlx::query_scalar("SELECT password_hash FROM admin_users WHERE id = 1")
            .fetch_one(&state.pool)
            .await?;
    if !verify_password(&input.current_password, &current_hash) {
        return Err(AppError::Unauthorized);
    }
    let password_hash = hash_password(&input.new_password).map_err(AppError::Internal)?;
    sqlx::query(
        "UPDATE admin_users SET username = ?, password_hash = ?, updated_at = ? WHERE id = 1",
    )
    .bind(username)
    .bind(password_hash)
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(&state.pool)
    .await?;
    state.sessions.write().await.clear();
    tracing::info!(username, "管理员账号凭据已更新，现有会话已清除");
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn logout(
    State(state): State<AppState>,
    request: Request,
) -> AppResult<impl IntoResponse> {
    if let Some(token) = bearer_token(&request) {
        state.sessions.write().await.remove(token);
    }
    Ok((
        [(SET_COOKIE, clear_session_cookie())],
        Json(serde_json::json!({ "ok": true })),
    ))
}

pub async fn require_auth(State(state): State<AppState>, request: Request, next: Next) -> Response {
    if is_token_authenticated(&state, bearer_token_owned(&request)).await {
        return next.run(request).await;
    }
    AppError::Unauthorized.into_response()
}

pub async fn allow_private_network_or_auth(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let client_ip = access::client_ip(&request);
    let token = bearer_token_owned(&request);
    match access::is_anonymous_allowed_for_ip(&state, client_ip).await {
        Ok(true) => next.run(request).await,
        Ok(false) if is_token_authenticated(&state, token).await => next.run(request).await,
        Ok(false) => AppError::Unauthorized.into_response(),
        Err(error) => error.into_response(),
    }
}

async fn is_token_authenticated(state: &AppState, token: Option<String>) -> bool {
    let Some(token) = token else {
        return false;
    };
    state.sessions.read().await.contains(&token)
}

fn bearer_token_owned(request: &Request) -> Option<String> {
    bearer_token(request).map(ToOwned::to_owned)
}

fn bearer_token(request: &Request) -> Option<&str> {
    request_bearer_token(request).or_else(|| cookie_token(request))
}

fn request_bearer_token(request: &Request) -> Option<&str> {
    request
        .headers()
        .get(AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}

fn cookie_token(request: &Request) -> Option<&str> {
    request
        .headers()
        .get("cookie")?
        .to_str()
        .ok()?
        .split(';')
        .map(str::trim)
        .find_map(|item| item.strip_prefix("service-compass-session="))
}

fn session_cookie(token: &str) -> String {
    format!("service-compass-session={token}; Path=/; HttpOnly; SameSite=Lax")
}

fn clear_session_cookie() -> String {
    "service-compass-session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0".into()
}

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| anyhow::anyhow!("无法生成管理员密码哈希: {error}"))
}

fn verify_password(password: &str, password_hash: &str) -> bool {
    PasswordHash::new(password_hash).is_ok_and(|hash| {
        Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_ok()
    })
}
