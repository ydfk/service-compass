use std::path::PathBuf;

use chrono::{Local, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tokio::time::{Duration, sleep};

use crate::{
    error::{AppError, AppResult},
    maintenance::archive,
    state::AppState,
};

#[derive(Clone, FromRow)]
pub struct BackupConfigRow {
    enabled: bool,
    schedule_time: String,
    target_type: String,
    local_dir: Option<String>,
    webdav_url: Option<String>,
    webdav_username: Option<String>,
    webdav_password_secret: Option<String>,
    retention_count: i64,
    last_run_at: Option<String>,
}

#[derive(Serialize)]
pub struct BackupConfigView {
    enabled: bool,
    schedule_time: String,
    target_type: String,
    local_dir: Option<String>,
    webdav_url: Option<String>,
    webdav_username: Option<String>,
    has_webdav_password: bool,
    retention_count: i64,
    last_run_at: Option<String>,
}

#[derive(Deserialize)]
pub struct BackupConfigInput {
    enabled: bool,
    schedule_time: String,
    target_type: String,
    local_dir: Option<String>,
    webdav_url: Option<String>,
    webdav_username: Option<String>,
    webdav_password: Option<String>,
    retention_count: i64,
}

pub fn start_scheduler(state: AppState) {
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(60)).await;
            if let Err(error) = tick(&state).await {
                tracing::warn!(?error, "计划备份检查失败");
            }
        }
    });
}

pub async fn get_config(state: &AppState) -> AppResult<BackupConfigView> {
    Ok(view(row(state).await?))
}

pub async fn update_config(
    state: &AppState,
    input: BackupConfigInput,
) -> AppResult<BackupConfigView> {
    validate(&input)?;
    let existing = row(state).await?;
    let password_secret = match input.webdav_password.as_deref() {
        Some(value) if !value.is_empty() => {
            Some(state.secrets.encrypt(value).map_err(AppError::Internal)?)
        }
        _ => existing.webdav_password_secret,
    };
    sqlx::query(
        "UPDATE backup_config SET enabled = ?, schedule_time = ?, target_type = ?, local_dir = ?, \
         webdav_url = ?, webdav_username = ?, webdav_password_secret = ?, retention_count = ?, updated_at = ? \
         WHERE id = 1",
    )
    .bind(input.enabled)
    .bind(input.schedule_time.trim())
    .bind(input.target_type.trim())
    .bind(clean(input.local_dir))
    .bind(clean(input.webdav_url))
    .bind(clean(input.webdav_username))
    .bind(password_secret)
    .bind(input.retention_count)
    .bind(Utc::now().to_rfc3339())
    .execute(&state.pool)
    .await?;
    get_config(state).await
}

pub async fn run_now(state: &AppState) -> AppResult<String> {
    let config = row(state).await?;
    run_with_config(state, &config).await
}

async fn tick(state: &AppState) -> AppResult<()> {
    let config = row(state).await?;
    if !config.enabled || !time_reached(&config) {
        return Ok(());
    }
    match run_with_config(state, &config).await {
        Ok(path) => tracing::info!(path, "计划备份完成"),
        Err(error) => tracing::warn!(?error, "计划备份失败"),
    }
    Ok(())
}

async fn run_with_config(state: &AppState, config: &BackupConfigRow) -> AppResult<String> {
    let archive = archive::create(state, "service-compass-backup").await?;
    let saved_path = match config.target_type.as_str() {
        "local" => save_local(config, &archive.filename, &archive.bytes).await?,
        "webdav" => save_webdav(state, config, &archive.filename, &archive.bytes).await?,
        _ => return Err(AppError::Validation("备份目标类型无效".into())),
    };
    sqlx::query("UPDATE backup_config SET last_run_at = ?, updated_at = ? WHERE id = 1")
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .execute(&state.pool)
        .await?;
    Ok(saved_path)
}

async fn save_local(config: &BackupConfigRow, filename: &str, bytes: &[u8]) -> AppResult<String> {
    let directory = config
        .local_dir
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::Validation("请填写本地备份目录".into()))?;
    let directory = PathBuf::from(directory);
    tokio::fs::create_dir_all(&directory)
        .await
        .map_err(anyhow::Error::from)?;
    let target = directory.join(filename);
    tokio::fs::write(&target, bytes)
        .await
        .map_err(anyhow::Error::from)?;
    cleanup_local(&directory, config.retention_count.max(1)).await;
    Ok(target.to_string_lossy().to_string())
}

async fn save_webdav(
    state: &AppState,
    config: &BackupConfigRow,
    filename: &str,
    bytes: &[u8],
) -> AppResult<String> {
    let base = config
        .webdav_url
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::Validation("请填写 WebDAV 地址".into()))?
        .trim_end_matches('/');
    let url = format!("{base}/{filename}");
    let mut request = reqwest::Client::new().put(&url).body(bytes.to_vec());
    if let Some(username) = config
        .webdav_username
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        let password = config
            .webdav_password_secret
            .as_deref()
            .and_then(|secret| state.secrets.decrypt(secret).ok())
            .unwrap_or_default();
        request = request.basic_auth(username, Some(password));
    }
    let response = request.send().await.map_err(anyhow::Error::from)?;
    if !response.status().is_success() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "WebDAV 上传失败：HTTP {}",
            response.status().as_u16()
        )));
    }
    Ok(url)
}

async fn cleanup_local(directory: &PathBuf, retention_count: i64) {
    let Ok(mut entries) = tokio::fs::read_dir(directory).await else {
        return;
    };
    let mut backups = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("service-compass-backup-") && name.ends_with(".zip") {
            backups.push((name, entry.path()));
        }
    }
    backups.sort_by(|left, right| right.0.cmp(&left.0));
    for (_, path) in backups.into_iter().skip(retention_count as usize) {
        let _ = tokio::fs::remove_file(path).await;
    }
}

async fn row(state: &AppState) -> AppResult<BackupConfigRow> {
    sqlx::query_as::<_, BackupConfigRow>(
        "SELECT enabled, schedule_time, target_type, local_dir, webdav_url, webdav_username, \
         webdav_password_secret, retention_count, last_run_at FROM backup_config WHERE id = 1",
    )
    .fetch_one(&state.pool)
    .await
    .map_err(AppError::from)
}

fn validate(input: &BackupConfigInput) -> AppResult<()> {
    if !matches!(input.target_type.as_str(), "local" | "webdav") {
        return Err(AppError::Validation("备份目标类型无效".into()));
    }
    if !valid_time(&input.schedule_time) {
        return Err(AppError::Validation("执行时间格式必须为 HH:mm".into()));
    }
    if !(1..=365).contains(&input.retention_count) {
        return Err(AppError::Validation(
            "备份保留份数必须在 1 到 365 之间".into(),
        ));
    }
    Ok(())
}

fn time_reached(config: &BackupConfigRow) -> bool {
    let now = Local::now();
    if config.schedule_time != now.format("%H:%M").to_string() {
        return false;
    }
    let today = now.format("%Y-%m-%d").to_string();
    !config
        .last_run_at
        .as_deref()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Local).format("%Y-%m-%d").to_string() == today)
        .unwrap_or(false)
}

fn valid_time(value: &str) -> bool {
    let mut parts = value.split(':');
    let Some(hour) = parts.next().and_then(|item| item.parse::<u32>().ok()) else {
        return false;
    };
    let Some(minute) = parts.next().and_then(|item| item.parse::<u32>().ok()) else {
        return false;
    };
    parts.next().is_none() && hour < 24 && minute < 60
}

fn view(row: BackupConfigRow) -> BackupConfigView {
    BackupConfigView {
        enabled: row.enabled,
        schedule_time: row.schedule_time,
        target_type: row.target_type,
        local_dir: row.local_dir,
        webdav_url: row.webdav_url,
        webdav_username: row.webdav_username,
        has_webdav_password: row.webdav_password_secret.is_some(),
        retention_count: row.retention_count,
        last_run_at: row.last_run_at,
    }
}

fn clean(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_owned())
        .filter(|item| !item.is_empty())
}
