use axum::{
    Json, Router,
    body::Body,
    extract::{Multipart, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use sqlx::FromRow;

use crate::{
    error::{AppError, AppResult},
    icon::local,
    maintenance::{archive, backup, restore},
    state::AppState,
};

#[derive(Deserialize, FromRow)]
struct IconRow {
    id: String,
    icon_type: String,
    icon_value: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/maintenance/export", get(export_config))
        .route("/api/maintenance/import", post(import_config))
        .route("/api/maintenance/icons/localize", post(localize_icons))
        .route(
            "/api/maintenance/backup-config",
            get(backup_config).put(update_backup_config),
        )
        .route("/api/maintenance/backup/run", post(run_backup))
}

async fn export_config(State(state): State<AppState>) -> AppResult<Response> {
    let archive = archive::create(&state, "service-compass-export").await?;
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/zip".to_string()),
            archive::content_disposition(&archive.filename),
        ],
        Body::from(archive.bytes),
    )
        .into_response())
}

async fn import_config(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> AppResult<Json<serde_json::Value>> {
    while let Some(field) = multipart.next_field().await.map_err(anyhow::Error::from)? {
        if field.name() != Some("file") {
            continue;
        }
        let bytes = field.bytes().await.map_err(anyhow::Error::from)?.to_vec();
        restore::prepare_pending_restore(&state.config, bytes).await?;
        return Ok(Json(serde_json::json!({
            "ok": true,
            "restart_required": true,
            "message": "配置包已写入待恢复区，请重启容器后生效"
        })));
    }
    Err(AppError::Validation("请选择要导入的 ZIP 配置包".into()))
}

async fn localize_icons(State(state): State<AppState>) -> AppResult<Json<serde_json::Value>> {
    let rows = sqlx::query_as::<_, IconRow>(
        "SELECT id, icon_type, icon_value FROM services WHERE icon_value IS NOT NULL AND icon_value <> ''",
    )
    .fetch_all(&state.pool)
    .await?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(anyhow::Error::from)?;
    let mut success = 0;
    let mut failed = Vec::new();
    for row in rows {
        match local::localize_icon_value(
            &state.config,
            &client,
            &row.icon_type,
            row.icon_value.as_deref(),
        )
        .await
        {
            Ok(Some((icon_type, icon_value))) => {
                sqlx::query("UPDATE services SET icon_type = ?, icon_value = ?, updated_at = ? WHERE id = ?")
                    .bind(icon_type)
                    .bind(icon_value)
                    .bind(chrono::Utc::now().to_rfc3339())
                    .bind(&row.id)
                    .execute(&state.pool)
                    .await?;
                success += 1;
            }
            Ok(None) => {}
            Err(error) => failed.push(serde_json::json!({
                "service_id": row.id,
                "reason": error.to_string()
            })),
        }
    }
    Ok(Json(serde_json::json!({
        "success": success,
        "failed": failed
    })))
}

async fn backup_config(State(state): State<AppState>) -> AppResult<Json<backup::BackupConfigView>> {
    Ok(Json(backup::get_config(&state).await?))
}

async fn update_backup_config(
    State(state): State<AppState>,
    Json(input): Json<backup::BackupConfigInput>,
) -> AppResult<Json<backup::BackupConfigView>> {
    Ok(Json(backup::update_config(&state, input).await?))
}

async fn run_backup(State(state): State<AppState>) -> AppResult<Json<serde_json::Value>> {
    let path = backup::run_now(&state).await?;
    Ok(Json(serde_json::json!({ "ok": true, "path": path })))
}
