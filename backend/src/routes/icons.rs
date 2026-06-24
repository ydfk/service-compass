use std::path::PathBuf;

use axum::{
    Json, Router,
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;

use crate::{
    error::{AppError, AppResult},
    icon::{favicon, selfhst},
    state::AppState,
};

#[derive(Deserialize)]
struct NameQuery {
    name: String,
}

#[derive(Deserialize)]
struct ReferenceQuery {
    reference: String,
}

#[derive(Deserialize)]
struct UrlQuery {
    url: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/icons/suggest", get(suggest))
        .route("/api/icons/test", get(test))
        .route("/api/icons/favicon", get(discover_favicon))
        .route("/api/icons/upload", post(upload))
}

pub fn public_router() -> Router<AppState> {
    Router::new().route("/api/icons/custom/{filename}", get(custom_icon))
}

async fn suggest(Query(query): Query<NameQuery>) -> Json<selfhst::IconSuggestion> {
    tracing::info!(name = %query.name, "生成 selfh.st 图标匹配建议");
    Json(selfhst::suggest(&query.name))
}

async fn test(Query(query): Query<ReferenceQuery>) -> AppResult<Json<serde_json::Value>> {
    let client = reqwest::Client::new();
    for url in selfhst::urls(&query.reference) {
        tracing::info!(reference = %query.reference, url, "测试 selfh.st 图标地址");
        if client
            .get(&url)
            .send()
            .await
            .is_ok_and(|response| response.status().is_success())
        {
            return Ok(Json(serde_json::json!({ "ok": true, "url": url })));
        }
    }
    Err(AppError::NotFound)
}

async fn discover_favicon(Query(query): Query<UrlQuery>) -> AppResult<Json<serde_json::Value>> {
    tracing::info!(service_url = %query.url, "开始发现服务 favicon");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(anyhow::Error::from)?;
    let urls = favicon::discover(&client, &query.url)
        .await
        .map_err(AppError::Internal)?;
    tracing::info!(service_url = %query.url, count = urls.len(), "服务 favicon 发现完成");
    Ok(Json(serde_json::json!({ "urls": urls })))
}

async fn upload(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> AppResult<Json<serde_json::Value>> {
    while let Some(field) = multipart.next_field().await.map_err(anyhow::Error::from)? {
        if field.name() != Some("file") {
            continue;
        }
        let content_type = field.content_type().unwrap_or_default().to_string();
        let extension = image_extension(&content_type)
            .ok_or_else(|| AppError::Validation("仅支持 PNG、JPEG、WebP 或 ICO 图标".into()))?;
        let bytes = field.bytes().await.map_err(anyhow::Error::from)?;
        if bytes.is_empty() || bytes.len() > 2 * 1024 * 1024 {
            return Err(AppError::Validation("图标大小必须在 2 MB 以内".into()));
        }
        let filename = format!("{}.{}", uuid::Uuid::new_v4(), extension);
        let directory = icon_directory(&state);
        tokio::fs::create_dir_all(&directory)
            .await
            .map_err(anyhow::Error::from)?;
        tokio::fs::write(directory.join(&filename), bytes)
            .await
            .map_err(anyhow::Error::from)?;
        tracing::info!(filename, content_type, "自定义服务图标上传完成");
        return Ok(Json(serde_json::json!({
            "url": format!("/api/icons/custom/{filename}")
        })));
    }
    Err(AppError::Validation("请选择图标文件".into()))
}

async fn custom_icon(
    State(state): State<AppState>,
    Path(filename): Path<String>,
) -> AppResult<Response> {
    if filename.contains(['/', '\\']) || filename.contains("..") {
        return Err(AppError::NotFound);
    }
    let path = icon_directory(&state).join(&filename);
    let bytes = tokio::fs::read(&path).await.map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            AppError::NotFound
        } else {
            AppError::Internal(error.into())
        }
    })?;
    let content_type = match path.extension().and_then(|value| value.to_str()) {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    };
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, content_type)],
        Body::from(bytes),
    )
        .into_response())
}

fn icon_directory(state: &AppState) -> PathBuf {
    state
        .config
        .secret_file
        .parent()
        .unwrap_or_else(|| std::path::Path::new("data"))
        .join("icons")
}

fn image_extension(content_type: &str) -> Option<&'static str> {
    match content_type {
        "image/png" => Some("png"),
        "image/jpeg" => Some("jpg"),
        "image/webp" => Some("webp"),
        "image/x-icon" | "image/vnd.microsoft.icon" => Some("ico"),
        _ => None,
    }
}
