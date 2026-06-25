use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::notification::{
        NotificationChannelInput, NotificationChannelRow, NotificationChannelView,
        NotificationEvent, NotificationRule, NotificationRuleInput,
    },
    notify,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/notifications/channels",
            get(list_channels).post(create_channel),
        )
        .route(
            "/api/notifications/channels/{id}",
            get(get_channel).put(update_channel).delete(remove_channel),
        )
        .route("/api/notifications/channels/{id}/test", post(test_channel))
        .route(
            "/api/notifications/rules",
            get(list_rules).post(create_rule),
        )
        .route(
            "/api/notifications/rules/{id}",
            axum::routing::put(update_rule).delete(remove_rule),
        )
}

async fn list_channels(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<NotificationChannelView>>> {
    let rows = sqlx::query_as::<_, NotificationChannelRow>(
        "SELECT * FROM notification_channels ORDER BY name",
    )
    .fetch_all(&state.pool)
    .await?;
    let views = rows
        .into_iter()
        .map(|row| channel_view(&state, row))
        .collect::<AppResult<Vec<_>>>()?;
    Ok(Json(views))
}

async fn get_channel(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<NotificationChannelView>> {
    let row = find_channel(&state, &id).await?;
    Ok(Json(channel_view(&state, row)?))
}

async fn create_channel(
    State(state): State<AppState>,
    Json(input): Json<NotificationChannelInput>,
) -> AppResult<Json<NotificationChannelView>> {
    validate_channel(&input, true)?;
    let id = Uuid::new_v4().to_string();
    save_channel(&state, &id, &input, true).await?;
    let row = find_channel(&state, &id).await?;
    Ok(Json(channel_view(&state, row)?))
}

async fn update_channel(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<NotificationChannelInput>,
) -> AppResult<Json<NotificationChannelView>> {
    validate_channel(&input, false)?;
    if !save_channel(&state, &id, &input, false).await? {
        return Err(AppError::NotFound);
    }
    let row = find_channel(&state, &id).await?;
    Ok(Json(channel_view(&state, row)?))
}

async fn remove_channel(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<Value>> {
    let result = sqlx::query("DELETE FROM notification_channels WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn test_channel(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<notify::SendResult>> {
    let channel = find_channel(&state, &id).await?;
    tracing::info!(channel_id = %id, channel_type = %channel.channel_type, "开始测试通知通道");
    let config = decrypt_channel_config(&state, &channel)
        .map_err(|_| AppError::Validation("通知通道配置无法解密，请重新保存通道配置".into()))?;
    let event = NotificationEvent {
        event_type: "test".into(),
        monitor_id: "test".into(),
        monitor_name: "通知测试".into(),
        service_name: Some("ServiceCompass".into()),
        status: "up".into(),
        message: "这是一条 ServiceCompass 测试通知".into(),
        target: None,
        latency_ms: None,
        status_code: None,
        checked_at: Utc::now().to_rfc3339(),
    };
    let client = reqwest::Client::new();
    let result = notify::send(&client, &channel.channel_type, &config, &event)
        .await
        .map_err(AppError::Internal)?;
    tracing::info!(channel_id = %id, status_code = result.status_code, "通知通道测试完成");
    Ok(Json(result))
}

async fn list_rules(State(state): State<AppState>) -> AppResult<Json<Vec<NotificationRule>>> {
    let rows = sqlx::query_as::<_, NotificationRule>(
        "SELECT * FROM notification_rules ORDER BY created_at",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

async fn create_rule(
    State(state): State<AppState>,
    Json(input): Json<NotificationRuleInput>,
) -> AppResult<Json<NotificationRule>> {
    validate_rule(&input)?;
    let id = Uuid::new_v4().to_string();
    save_rule(&state, &id, &input, true).await?;
    Ok(Json(find_rule(&state, &id).await?))
}

async fn update_rule(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<NotificationRuleInput>,
) -> AppResult<Json<NotificationRule>> {
    validate_rule(&input)?;
    if !save_rule(&state, &id, &input, false).await? {
        return Err(AppError::NotFound);
    }
    Ok(Json(find_rule(&state, &id).await?))
}

async fn remove_rule(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<Value>> {
    let result = sqlx::query("DELETE FROM notification_rules WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn find_channel(state: &AppState, id: &str) -> AppResult<NotificationChannelRow> {
    sqlx::query_as::<_, NotificationChannelRow>("SELECT * FROM notification_channels WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)
}

fn channel_view(
    state: &AppState,
    row: NotificationChannelRow,
) -> AppResult<NotificationChannelView> {
    let config = match decrypt_channel_config(state, &row) {
        Ok(config) => config,
        Err(error) => {
            tracing::warn!(
                channel_id = %row.id,
                channel_type = %row.channel_type,
                ?error,
                "通知通道配置无法解密，返回未配置状态"
            );
            return Ok(NotificationChannelView {
                id: row.id,
                name: row.name,
                channel_type: row.channel_type,
                enabled: row.enabled,
                configured: false,
                config: serde_json::json!({}),
            });
        }
    };
    Ok(NotificationChannelView {
        id: row.id,
        name: row.name,
        channel_type: row.channel_type,
        enabled: row.enabled,
        configured: true,
        config,
    })
}

fn decrypt_channel_config(state: &AppState, row: &NotificationChannelRow) -> anyhow::Result<Value> {
    let decrypted = state.secrets.decrypt(&row.config_secret)?;
    serde_json::from_str(&decrypted).map_err(Into::into)
}

async fn save_channel(
    state: &AppState,
    id: &str,
    input: &NotificationChannelInput,
    insert: bool,
) -> AppResult<bool> {
    let secret = if let Some(config) = &input.config {
        state
            .secrets
            .encrypt(&config.to_string())
            .map_err(AppError::Internal)?
    } else if insert {
        return Err(AppError::Validation("通知配置不能为空".into()));
    } else {
        sqlx::query_scalar("SELECT config_secret FROM notification_channels WHERE id = ?")
            .bind(id)
            .fetch_optional(&state.pool)
            .await?
            .ok_or(AppError::NotFound)?
    };
    let now = Utc::now().to_rfc3339();
    let result = if insert {
        sqlx::query(
            "INSERT INTO notification_channels (id, name, channel_type, enabled, config_secret, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(input.name.trim())
        .bind(&input.channel_type)
        .bind(input.enabled)
        .bind(secret)
        .bind(&now)
        .bind(&now)
        .execute(&state.pool)
        .await?
    } else {
        sqlx::query(
            "UPDATE notification_channels SET name = ?, channel_type = ?, enabled = ?, \
             config_secret = ?, updated_at = ? WHERE id = ?",
        )
        .bind(input.name.trim())
        .bind(&input.channel_type)
        .bind(input.enabled)
        .bind(secret)
        .bind(&now)
        .bind(id)
        .execute(&state.pool)
        .await?
    };
    Ok(result.rows_affected() > 0)
}

async fn find_rule(state: &AppState, id: &str) -> AppResult<NotificationRule> {
    sqlx::query_as::<_, NotificationRule>("SELECT * FROM notification_rules WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)
}

async fn save_rule(
    state: &AppState,
    id: &str,
    input: &NotificationRuleInput,
    insert: bool,
) -> AppResult<bool> {
    let now = Utc::now().to_rfc3339();
    let result = if insert {
        sqlx::query(
            "INSERT INTO notification_rules (id, monitor_id, channel_id, notify_on_down, notify_on_recovery, \
             notify_on_warning, cooldown_sec, enabled, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&input.monitor_id)
        .bind(&input.channel_id)
        .bind(input.notify_on_down)
        .bind(input.notify_on_recovery)
        .bind(input.notify_on_warning)
        .bind(input.cooldown_sec)
        .bind(input.enabled)
        .bind(&now)
        .bind(&now)
        .execute(&state.pool)
        .await?
    } else {
        sqlx::query(
            "UPDATE notification_rules SET monitor_id = ?, channel_id = ?, notify_on_down = ?, \
             notify_on_recovery = ?, notify_on_warning = ?, cooldown_sec = ?, enabled = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&input.monitor_id)
        .bind(&input.channel_id)
        .bind(input.notify_on_down)
        .bind(input.notify_on_recovery)
        .bind(input.notify_on_warning)
        .bind(input.cooldown_sec)
        .bind(input.enabled)
        .bind(&now)
        .bind(id)
        .execute(&state.pool)
        .await?
    };
    Ok(result.rows_affected() > 0)
}

fn validate_channel(input: &NotificationChannelInput, creating: bool) -> AppResult<()> {
    if input.name.trim().is_empty() {
        return Err(AppError::Validation("通知名称不能为空".into()));
    }
    if !matches!(
        input.channel_type.as_str(),
        "bark" | "webhook" | "synology_chat"
    ) {
        return Err(AppError::Validation("通知类型无效".into()));
    }
    if creating && input.config.is_none() {
        return Err(AppError::Validation("通知配置不能为空".into()));
    }
    if input.channel_type == "synology_chat"
        && let Some(config) = &input.config
    {
        crate::notify::synology_chat::validate_config(config)
            .map_err(|error| AppError::Validation(error.to_string()))?;
    }
    Ok(())
}

fn validate_rule(input: &NotificationRuleInput) -> AppResult<()> {
    if input.channel_id.is_empty() || input.cooldown_sec < 0 {
        return Err(AppError::Validation("通知通道或冷却时间无效".into()));
    }
    Ok(())
}
