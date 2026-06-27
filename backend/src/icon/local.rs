use std::{path::PathBuf, sync::Arc};

use reqwest::Client;
use reqwest::header;
use uuid::Uuid;

use crate::{
    config::Config,
    error::{AppError, AppResult},
    icon::selfhst,
};

pub fn icon_directory(config: &Config) -> PathBuf {
    config
        .secret_file
        .parent()
        .unwrap_or_else(|| std::path::Path::new("data"))
        .join("icons")
}

pub fn custom_icon_url(filename: &str) -> String {
    format!("/api/icons/custom/{filename}")
}

pub async fn save_custom_icon_bytes(
    config: &Config,
    prefix: &str,
    extension: &str,
    bytes: impl AsRef<[u8]>,
) -> AppResult<String> {
    let bytes = bytes.as_ref();
    if bytes.is_empty() || bytes.len() > 2 * 1024 * 1024 {
        return Err(AppError::Validation("图标大小必须在 2 MB 以内".into()));
    }
    let directory = icon_directory(config);
    tokio::fs::create_dir_all(&directory)
        .await
        .map_err(anyhow::Error::from)?;
    let filename = format!("{prefix}-{}.{}", Uuid::new_v4(), extension);
    tokio::fs::write(directory.join(&filename), bytes)
        .await
        .map_err(anyhow::Error::from)?;
    Ok(custom_icon_url(&filename))
}

pub async fn download_remote_icon(
    client: &Client,
    config: &Config,
    url: &str,
    prefix: &str,
) -> AppResult<Option<String>> {
    let response = match client.get(url).send().await {
        Ok(response) if response.status().is_success() => response,
        Ok(response) => {
            tracing::warn!(url, status = %response.status(), "远程图标下载失败");
            return Ok(None);
        }
        Err(error) => {
            tracing::warn!(url, ?error, "远程图标请求失败");
            return Ok(None);
        }
    };
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .unwrap_or_default()
        .to_string();
    let extension = image_extension(&content_type)
        .or_else(|| extension_from_url(url))
        .ok_or_else(|| AppError::Validation("图标格式不支持".into()))?;
    let bytes = response.bytes().await.map_err(anyhow::Error::from)?;
    save_custom_icon_bytes(config, prefix, extension, bytes)
        .await
        .map(Some)
}

pub async fn localize_icon_value(
    config: &Arc<Config>,
    client: &Client,
    icon_type: &str,
    icon_value: Option<&str>,
) -> AppResult<Option<(String, String)>> {
    let Some(value) = icon_value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if value.starts_with("/api/icons/custom/") {
        return Ok(None);
    }
    if icon_type == "selfhst" {
        for url in selfhst::urls(value) {
            if let Some(local_url) = download_remote_icon(client, config, &url, "selfhst").await? {
                return Ok(Some(("upload".into(), local_url)));
            }
        }
        return Ok(None);
    }
    if is_remote_url(value)
        && let Some(local_url) = download_remote_icon(client, config, value, "remote").await?
    {
        return Ok(Some(("upload".into(), local_url)));
    }
    Ok(None)
}

pub fn image_extension(content_type: &str) -> Option<&'static str> {
    match content_type {
        "image/png" => Some("png"),
        "image/jpeg" => Some("jpg"),
        "image/webp" => Some("webp"),
        "image/svg+xml" => Some("svg"),
        "image/x-icon" | "image/vnd.microsoft.icon" => Some("ico"),
        _ => None,
    }
}

pub fn extension_from_url(url: &str) -> Option<&'static str> {
    let lower = url.split('?').next().unwrap_or(url).to_lowercase();
    if lower.ends_with(".svg") {
        Some("svg")
    } else if lower.ends_with(".png") {
        Some("png")
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        Some("jpg")
    } else if lower.ends_with(".webp") {
        Some("webp")
    } else if lower.ends_with(".ico") {
        Some("ico")
    } else {
        None
    }
}

fn is_remote_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}
