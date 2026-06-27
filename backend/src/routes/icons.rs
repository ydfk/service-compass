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
    icon::{favicon, local, selfhst},
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
struct FaviconInput {
    url: String,
    auth_type: Option<String>,
    auth_username: Option<String>,
    auth_password: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/icons/suggest", get(suggest))
        .route("/api/icons/test", get(test))
        .route("/api/icons/favicon", post(discover_favicon))
        .route("/api/icons/upload", post(upload))
}

pub fn public_router() -> Router<AppState> {
    Router::new().route("/api/icons/custom/{filename}", get(custom_icon))
}

async fn suggest(Query(query): Query<NameQuery>) -> Json<selfhst::IconSuggestion> {
    tracing::info!(name = %query.name, "生成 selfh.st 图标匹配建议");
    Json(selfhst::suggest(&query.name))
}

async fn test(
    State(state): State<AppState>,
    Query(query): Query<ReferenceQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .map_err(anyhow::Error::from)?;
    for url in selfhst::urls(&query.reference) {
        tracing::info!(reference = %query.reference, url, "测试 selfh.st 图标地址");
        if let Some(local_url) =
            local::download_remote_icon(&client, &state.config, &url, "selfhst").await?
        {
            tracing::info!(reference = %query.reference, local_url, "selfh.st 图标已下载到本地");
            return Ok(Json(serde_json::json!({ "ok": true, "url": local_url })));
        }
    }
    Err(AppError::NotFound)
}

async fn discover_favicon(Json(input): Json<FaviconInput>) -> AppResult<Json<serde_json::Value>> {
    let url = input.url.trim();
    if url.is_empty() {
        return Err(AppError::Validation("服务地址不能为空".into()));
    }
    tracing::info!(service_url = %url, "开始发现服务 favicon");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(anyhow::Error::from)?;
    let auth = favicon_auth(&input);
    let urls = match favicon::discover(&client, url, auth.as_ref()).await {
        Ok(urls) => urls,
        Err(error) if error.to_string().contains("未发现 favicon") => Vec::new(),
        Err(error) if error.to_string().contains("服务 URL 无效") => {
            return Err(AppError::Validation("服务地址格式无效".into()));
        }
        Err(error) => return Err(AppError::Internal(error)),
    };
    tracing::info!(service_url = %url, count = urls.len(), "服务 favicon 发现完成");
    Ok(Json(serde_json::json!({ "urls": urls })))
}

fn favicon_auth(input: &FaviconInput) -> Option<favicon::FaviconAuth> {
    if input.auth_type.as_deref() != Some("basic") {
        return None;
    }
    Some(favicon::FaviconAuth {
        username: input.auth_username.clone().unwrap_or_default(),
        password: input.auth_password.clone().unwrap_or_default(),
    })
    .filter(|auth| !auth.username.trim().is_empty() || !auth.password.is_empty())
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
        let extension = local::image_extension(&content_type).ok_or_else(|| {
            AppError::Validation("仅支持 PNG、JPEG、WebP、SVG 或 ICO 图标".into())
        })?;
        let bytes = field.bytes().await.map_err(anyhow::Error::from)?;
        let url = local::save_custom_icon_bytes(&state.config, "upload", extension, bytes).await?;
        tracing::info!(url, content_type, "自定义服务图标上传完成");
        return Ok(Json(serde_json::json!({
            "url": url
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
    let path = local::icon_directory(&state.config).join(&filename);
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
        Some("svg") => "image/svg+xml",
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
