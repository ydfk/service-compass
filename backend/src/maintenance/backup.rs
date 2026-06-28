use std::path::PathBuf;

use axum::http::StatusCode;
use chrono::{Local, Utc};
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tokio::time::{Duration, sleep};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    maintenance::{archive, oss},
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
    aliyun_oss_endpoint: Option<String>,
    aliyun_oss_region: Option<String>,
    aliyun_oss_bucket: Option<String>,
    aliyun_oss_prefix: Option<String>,
    aliyun_oss_access_key_id: Option<String>,
    aliyun_oss_access_key_secret: Option<String>,
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
    aliyun_oss_endpoint: Option<String>,
    aliyun_oss_region: Option<String>,
    aliyun_oss_bucket: Option<String>,
    aliyun_oss_prefix: Option<String>,
    aliyun_oss_access_key_id: Option<String>,
    has_aliyun_oss_access_key_secret: bool,
    retention_count: i64,
    last_run_at: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct BackupConfigInput {
    enabled: bool,
    schedule_time: String,
    target_type: String,
    local_dir: Option<String>,
    webdav_url: Option<String>,
    webdav_username: Option<String>,
    webdav_password: Option<String>,
    aliyun_oss_endpoint: Option<String>,
    aliyun_oss_region: Option<String>,
    aliyun_oss_bucket: Option<String>,
    aliyun_oss_prefix: Option<String>,
    aliyun_oss_access_key_id: Option<String>,
    aliyun_oss_access_key_secret: Option<String>,
    retention_count: i64,
}

#[derive(Clone, FromRow, Serialize)]
pub struct BackupRunView {
    id: String,
    run_type: String,
    target_type: String,
    status: String,
    filename: Option<String>,
    file_size: Option<i64>,
    started_at: String,
    finished_at: Option<String>,
    http_status_code: Option<i64>,
    response_summary: Option<String>,
    error_message: Option<String>,
}

#[derive(Serialize)]
pub struct BackupRunsPage {
    items: Vec<BackupRunView>,
    total: i64,
}

struct TargetResult {
    location: String,
    http_status_code: Option<i64>,
    response_summary: Option<String>,
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
    let webdav_password_secret = secret_or_existing(
        state,
        input.webdav_password.as_deref(),
        existing.webdav_password_secret,
    )?;
    let aliyun_oss_access_key_secret = secret_or_existing(
        state,
        input.aliyun_oss_access_key_secret.as_deref(),
        existing.aliyun_oss_access_key_secret,
    )?;
    sqlx::query(
        "UPDATE backup_config SET enabled = ?, schedule_time = ?, target_type = ?, local_dir = ?, \
         webdav_url = ?, webdav_username = ?, webdav_password_secret = ?, aliyun_oss_endpoint = ?, \
         aliyun_oss_region = ?, aliyun_oss_bucket = ?, aliyun_oss_prefix = ?, aliyun_oss_access_key_id = ?, \
         aliyun_oss_access_key_secret = ?, retention_count = ?, updated_at = ? WHERE id = 1",
    )
    .bind(input.enabled)
    .bind(input.schedule_time.trim())
    .bind(input.target_type.trim())
    .bind(clean(input.local_dir))
    .bind(clean(input.webdav_url))
    .bind(clean(input.webdav_username))
    .bind(webdav_password_secret)
    .bind(clean(input.aliyun_oss_endpoint))
    .bind(clean(input.aliyun_oss_region))
    .bind(clean(input.aliyun_oss_bucket))
    .bind(clean(input.aliyun_oss_prefix))
    .bind(clean(input.aliyun_oss_access_key_id))
    .bind(aliyun_oss_access_key_secret)
    .bind(input.retention_count)
    .bind(Utc::now().to_rfc3339())
    .execute(&state.pool)
    .await?;
    get_config(state).await
}

pub async fn run_now(state: &AppState) -> AppResult<String> {
    let config = row(state).await?;
    run_with_config(state, &config, "manual").await
}

pub async fn test_target(state: &AppState, input: BackupConfigInput) -> AppResult<String> {
    validate(&input)?;
    let config = input_to_row(state, input).await?;
    let run_id = record_start(state, "test", &config.target_type).await?;
    let result = test_with_config(state, &config).await;
    finish_run(state, &run_id, &config, &result, None, None).await;
    result.map(|item| item.location)
}

pub async fn list_runs(state: &AppState, page: i64, page_size: i64) -> AppResult<BackupRunsPage> {
    let page = page.max(1);
    let page_size = page_size.clamp(1, 100);
    let offset = (page - 1) * page_size;
    let items = sqlx::query_as::<_, BackupRunView>(
        "SELECT * FROM backup_runs ORDER BY started_at DESC LIMIT ? OFFSET ?",
    )
    .bind(page_size)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;
    let total = sqlx::query_scalar("SELECT COUNT(*) FROM backup_runs")
        .fetch_one(&state.pool)
        .await?;
    Ok(BackupRunsPage { items, total })
}

async fn tick(state: &AppState) -> AppResult<()> {
    let config = row(state).await?;
    if !config.enabled || !time_reached(&config) {
        return Ok(());
    }
    match run_with_config(state, &config, "scheduled").await {
        Ok(path) => tracing::info!(path, "计划备份完成"),
        Err(error) => tracing::warn!(?error, "计划备份失败"),
    }
    Ok(())
}

async fn run_with_config(
    state: &AppState,
    config: &BackupConfigRow,
    run_type: &str,
) -> AppResult<String> {
    let archive = archive::create(state, "service-compass-backup").await?;
    let run_id = record_start(state, run_type, &config.target_type).await?;
    let result = save_archive(state, config, &archive.filename, archive.bytes).await;
    finish_run(
        state,
        &run_id,
        config,
        &result,
        Some(archive.filename),
        result.as_ref().ok().map(|item| item.0),
    )
    .await;
    let (_, target) = result?;
    sqlx::query("UPDATE backup_config SET last_run_at = ?, updated_at = ? WHERE id = 1")
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .execute(&state.pool)
        .await?;
    let _ = cleanup_runs(state).await;
    Ok(target.location)
}

async fn save_archive(
    state: &AppState,
    config: &BackupConfigRow,
    filename: &str,
    bytes: Vec<u8>,
) -> AppResult<(i64, TargetResult)> {
    let size = bytes.len() as i64;
    let result = match config.target_type.as_str() {
        "local" => save_local(config, filename, &bytes).await?,
        "webdav" => save_webdav(state, config, filename, bytes).await?,
        "aliyun_oss" => save_oss(state, config, filename, bytes).await?,
        _ => return Err(AppError::Validation("备份目标类型无效".into())),
    };
    Ok((size, result))
}

async fn test_with_config(state: &AppState, config: &BackupConfigRow) -> AppResult<TargetResult> {
    let filename = format!("service-compass-test-{}.txt", Uuid::new_v4());
    let bytes = b"ServiceCompass backup target test".to_vec();
    match config.target_type.as_str() {
        "local" => {
            let result = save_local(config, &filename, &bytes).await?;
            let _ = tokio::fs::remove_file(&result.location).await;
            Ok(result)
        }
        "webdav" => test_webdav(state, config, &filename, bytes).await,
        "aliyun_oss" => test_oss(state, config, &filename, bytes).await,
        _ => Err(AppError::Validation("备份目标类型无效".into())),
    }
}

async fn save_local(
    config: &BackupConfigRow,
    filename: &str,
    bytes: &[u8],
) -> AppResult<TargetResult> {
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
    Ok(TargetResult {
        location: target.to_string_lossy().to_string(),
        http_status_code: None,
        response_summary: None,
    })
}

async fn save_webdav(
    state: &AppState,
    config: &BackupConfigRow,
    filename: &str,
    bytes: Vec<u8>,
) -> AppResult<TargetResult> {
    let client = Client::new();
    put_webdav(state, config, &client, filename, bytes).await
}

async fn test_webdav(
    state: &AppState,
    config: &BackupConfigRow,
    filename: &str,
    bytes: Vec<u8>,
) -> AppResult<TargetResult> {
    let client = Client::new();
    let base = webdav_base(config)?;
    let propfind = webdav_request(
        state,
        config,
        &client,
        Method::from_bytes(b"PROPFIND").unwrap(),
        &base,
    )
    .header("Depth", "0")
    .send()
    .await
    .map_err(anyhow::Error::from)?;
    if propfind.status() == StatusCode::NOT_FOUND {
        let mkcol = webdav_request(
            state,
            config,
            &client,
            Method::from_bytes(b"MKCOL").unwrap(),
            &base,
        )
        .send()
        .await
        .map_err(anyhow::Error::from)?;
        ensure_http_success("WebDAV 创建目录失败", mkcol).await?;
    } else {
        ensure_http_success("WebDAV 目录不可用", propfind).await?;
    }
    let result = put_webdav(state, config, &client, filename, bytes).await?;
    let delete_url = format!("{}/{}", base.trim_end_matches('/'), filename);
    let _ = webdav_request(state, config, &client, Method::DELETE, &delete_url)
        .send()
        .await;
    Ok(result)
}

async fn put_webdav(
    state: &AppState,
    config: &BackupConfigRow,
    client: &Client,
    filename: &str,
    bytes: Vec<u8>,
) -> AppResult<TargetResult> {
    let base = webdav_base(config)?;
    let url = format!("{}/{}", base.trim_end_matches('/'), filename);
    let response = webdav_request(state, config, client, Method::PUT, &url)
        .body(bytes)
        .send()
        .await
        .map_err(anyhow::Error::from)?;
    let status_code = response.status().as_u16();
    let summary = response
        .text()
        .await
        .unwrap_or_default()
        .chars()
        .take(512)
        .collect::<String>();
    if !(200..300).contains(&status_code) && status_code != 201 && status_code != 204 {
        return Err(AppError::Internal(anyhow::anyhow!(
            "WebDAV 上传失败：HTTP {} {}",
            status_code,
            summary
        )));
    }
    Ok(TargetResult {
        location: url,
        http_status_code: Some(i64::from(status_code)),
        response_summary: (!summary.is_empty()).then_some(summary),
    })
}

async fn save_oss(
    state: &AppState,
    config: &BackupConfigRow,
    filename: &str,
    bytes: Vec<u8>,
) -> AppResult<TargetResult> {
    let client = Client::new();
    let target = oss_target(state, config)?;
    let response = oss::put_object(&client, &target, filename, bytes).await?;
    ensure_oss_success("阿里云 OSS 上传失败", response)
}

async fn test_oss(
    state: &AppState,
    config: &BackupConfigRow,
    filename: &str,
    bytes: Vec<u8>,
) -> AppResult<TargetResult> {
    let client = Client::new();
    let target = oss_target(state, config)?;
    let response = oss::put_object(&client, &target, filename, bytes).await?;
    let result = ensure_oss_success("阿里云 OSS 测试上传失败", response)?;
    let _ = oss::delete_object(&client, &target, filename).await;
    Ok(result)
}

async fn ensure_http_success(label: &str, response: reqwest::Response) -> AppResult<()> {
    let status = response.status();
    let summary = response
        .text()
        .await
        .unwrap_or_default()
        .chars()
        .take(512)
        .collect::<String>();
    if !status.is_success() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "{}：HTTP {} {}",
            label,
            status.as_u16(),
            summary
        )));
    }
    Ok(())
}

fn ensure_oss_success(label: &str, response: oss::OssResponse) -> AppResult<TargetResult> {
    if !(200..300).contains(&response.status_code) {
        return Err(AppError::Internal(anyhow::anyhow!(
            "{}：HTTP {} {}",
            label,
            response.status_code,
            response.response_summary
        )));
    }
    Ok(TargetResult {
        location: response.url,
        http_status_code: Some(i64::from(response.status_code)),
        response_summary: (!response.response_summary.is_empty())
            .then_some(response.response_summary),
    })
}

fn webdav_request(
    state: &AppState,
    config: &BackupConfigRow,
    client: &Client,
    method: Method,
    url: &str,
) -> reqwest::RequestBuilder {
    let request = client.request(method, url);
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
        return request.basic_auth(username, Some(password));
    }
    request
}

fn webdav_base(config: &BackupConfigRow) -> AppResult<String> {
    config
        .webdav_url
        .as_deref()
        .map(|value| value.trim().trim_end_matches('/').to_owned())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::Validation("请填写 WebDAV 目录地址".into()))
}

fn oss_target(state: &AppState, config: &BackupConfigRow) -> AppResult<oss::OssTarget> {
    Ok(oss::OssTarget {
        endpoint: required(config.aliyun_oss_endpoint.as_deref(), "请填写 OSS Endpoint")?,
        region: required(config.aliyun_oss_region.as_deref(), "请填写 OSS Region")?,
        bucket: required(config.aliyun_oss_bucket.as_deref(), "请填写 OSS Bucket")?,
        prefix: config.aliyun_oss_prefix.clone(),
        access_key_id: required(
            config.aliyun_oss_access_key_id.as_deref(),
            "请填写 OSS AccessKey ID",
        )?,
        access_key_secret: config
            .aliyun_oss_access_key_secret
            .as_deref()
            .and_then(|secret| state.secrets.decrypt(secret).ok())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| AppError::Validation("请填写 OSS AccessKey Secret".into()))?,
    })
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
         webdav_password_secret, aliyun_oss_endpoint, aliyun_oss_region, aliyun_oss_bucket, \
         aliyun_oss_prefix, aliyun_oss_access_key_id, aliyun_oss_access_key_secret, retention_count, \
         last_run_at FROM backup_config WHERE id = 1",
    )
    .fetch_one(&state.pool)
    .await
    .map_err(AppError::from)
}

async fn input_to_row(state: &AppState, input: BackupConfigInput) -> AppResult<BackupConfigRow> {
    let existing = row(state).await.ok();
    Ok(BackupConfigRow {
        enabled: input.enabled,
        schedule_time: input.schedule_time,
        target_type: input.target_type,
        local_dir: clean(input.local_dir),
        webdav_url: clean(input.webdav_url),
        webdav_username: clean(input.webdav_username),
        webdav_password_secret: secret_or_existing(
            state,
            input.webdav_password.as_deref(),
            existing
                .as_ref()
                .and_then(|item| item.webdav_password_secret.clone()),
        )?,
        aliyun_oss_endpoint: clean(input.aliyun_oss_endpoint),
        aliyun_oss_region: clean(input.aliyun_oss_region),
        aliyun_oss_bucket: clean(input.aliyun_oss_bucket),
        aliyun_oss_prefix: clean(input.aliyun_oss_prefix),
        aliyun_oss_access_key_id: clean(input.aliyun_oss_access_key_id),
        aliyun_oss_access_key_secret: secret_or_existing(
            state,
            input.aliyun_oss_access_key_secret.as_deref(),
            existing
                .as_ref()
                .and_then(|item| item.aliyun_oss_access_key_secret.clone()),
        )?,
        retention_count: input.retention_count,
        last_run_at: existing.and_then(|item| item.last_run_at),
    })
}

fn validate(input: &BackupConfigInput) -> AppResult<()> {
    if !matches!(
        input.target_type.as_str(),
        "local" | "webdav" | "aliyun_oss"
    ) {
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

async fn record_start(state: &AppState, run_type: &str, target_type: &str) -> AppResult<String> {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO backup_runs (id, run_type, target_type, status, started_at) VALUES (?, ?, ?, 'running', ?)",
    )
    .bind(&id)
    .bind(run_type)
    .bind(target_type)
    .bind(Utc::now().to_rfc3339())
    .execute(&state.pool)
    .await?;
    Ok(id)
}

async fn finish_run(
    state: &AppState,
    run_id: &str,
    config: &BackupConfigRow,
    result: &AppResult<impl ResultInfo>,
    filename: Option<String>,
    file_size: Option<i64>,
) {
    let (status, location, http_status_code, response_summary, error_message) = match result {
        Ok(value) => (
            "success",
            Some(value.location()),
            value.http_status_code(),
            value.response_summary(),
            None,
        ),
        Err(error) => (
            "failed",
            None,
            http_status_from_error(&error.to_string()),
            None,
            Some(error.to_string()),
        ),
    };
    let _ = sqlx::query(
        "UPDATE backup_runs SET status = ?, filename = ?, file_size = ?, finished_at = ?, http_status_code = ?, \
         response_summary = ?, error_message = ? WHERE id = ?",
    )
    .bind(status)
    .bind(filename.or(location))
    .bind(file_size)
    .bind(Utc::now().to_rfc3339())
    .bind(http_status_code)
    .bind(response_summary)
    .bind(error_message)
    .bind(run_id)
    .execute(&state.pool)
    .await;
    let _ = cleanup_runs(state).await;
    tracing::info!(
        run_id,
        target_type = %config.target_type,
        status,
        "备份任务记录已更新"
    );
}

trait ResultInfo {
    fn location(&self) -> String;
    fn http_status_code(&self) -> Option<i64>;
    fn response_summary(&self) -> Option<String>;
}

impl ResultInfo for TargetResult {
    fn location(&self) -> String {
        self.location.clone()
    }

    fn http_status_code(&self) -> Option<i64> {
        self.http_status_code
    }

    fn response_summary(&self) -> Option<String> {
        self.response_summary.clone()
    }
}

impl ResultInfo for (i64, TargetResult) {
    fn location(&self) -> String {
        self.1.location.clone()
    }

    fn http_status_code(&self) -> Option<i64> {
        self.1.http_status_code
    }

    fn response_summary(&self) -> Option<String> {
        self.1.response_summary.clone()
    }
}

async fn cleanup_runs(state: &AppState) -> AppResult<()> {
    let retention_days = setting_i64(state, "log_retention_days", 30).await?;
    let cutoff = (Utc::now() - chrono::Duration::days(retention_days.max(1))).to_rfc3339();
    sqlx::query("DELETE FROM backup_runs WHERE started_at < ?")
        .bind(cutoff)
        .execute(&state.pool)
        .await?;
    Ok(())
}

async fn setting_i64(state: &AppState, key: &str, fallback: i64) -> AppResult<i64> {
    let value: Option<String> = sqlx::query_scalar("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(&state.pool)
        .await?;
    Ok(value
        .and_then(|value| value.parse().ok())
        .unwrap_or(fallback))
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
        aliyun_oss_endpoint: row.aliyun_oss_endpoint,
        aliyun_oss_region: row.aliyun_oss_region,
        aliyun_oss_bucket: row.aliyun_oss_bucket,
        aliyun_oss_prefix: row.aliyun_oss_prefix,
        aliyun_oss_access_key_id: row.aliyun_oss_access_key_id,
        has_aliyun_oss_access_key_secret: row.aliyun_oss_access_key_secret.is_some(),
        retention_count: row.retention_count,
        last_run_at: row.last_run_at,
    }
}

fn secret_or_existing(
    state: &AppState,
    value: Option<&str>,
    existing: Option<String>,
) -> AppResult<Option<String>> {
    match value {
        Some(value) if !value.is_empty() => Ok(Some(
            state.secrets.encrypt(value).map_err(AppError::Internal)?,
        )),
        _ => Ok(existing),
    }
}

fn clean(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_owned())
        .filter(|item| !item.is_empty())
}

fn required(value: Option<&str>, message: &str) -> AppResult<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| AppError::Validation(message.into()))
}

fn http_status_from_error(message: &str) -> Option<i64> {
    let marker = "HTTP ";
    let start = message.find(marker)? + marker.len();
    let digits = message[start..]
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>();
    digits.parse().ok()
}
