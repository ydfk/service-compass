use std::{io::Write, path::PathBuf};

use axum::http::header;
use chrono::Utc;
use serde::Serialize;
use sha2::{Digest, Sha256};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

use crate::{
    config::Config,
    error::{AppError, AppResult},
    icon::local,
    state::AppState,
};

pub struct ExportArchive {
    pub filename: String,
    pub bytes: Vec<u8>,
}

#[derive(Serialize)]
struct Manifest {
    app: &'static str,
    version: &'static str,
    exported_at: String,
    files: Vec<ManifestFile>,
}

#[derive(Serialize)]
struct ManifestFile {
    path: String,
    size: u64,
    sha256: String,
}

pub async fn create(state: &AppState, filename_prefix: &str) -> AppResult<ExportArchive> {
    let _ = sqlx::query("PRAGMA wal_checkpoint(FULL)")
        .execute(&state.pool)
        .await;
    let config = (*state.config).clone();
    let filename = format!(
        "{filename_prefix}-{}.zip",
        Utc::now().format("%Y%m%d-%H%M%S")
    );
    let bytes = tokio::task::spawn_blocking(move || build_zip(&config))
        .await
        .map_err(anyhow::Error::from)??;
    Ok(ExportArchive { filename, bytes })
}

pub fn database_file(config: &Config) -> AppResult<PathBuf> {
    let Some(path) = config.database_url.strip_prefix("sqlite:") else {
        return Err(AppError::Validation("仅支持导出 SQLite 文件数据库".into()));
    };
    let path = path.split('?').next().unwrap_or(path);
    if path == ":memory:" || path.is_empty() {
        return Err(AppError::Validation("内存数据库无法导出".into()));
    }
    Ok(PathBuf::from(path))
}

pub fn content_disposition(filename: &str) -> (header::HeaderName, String) {
    (
        header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{filename}\""),
    )
}

fn build_zip(config: &Config) -> AppResult<Vec<u8>> {
    let mut files = Vec::new();
    files.push(read_required_file(
        "service-compass.db",
        database_file(config)?,
    )?);
    files.push(read_required_file(
        "secret.key",
        config.secret_file.clone(),
    )?);
    collect_icons(&mut files, local::icon_directory(config), "icons")?;

    let manifest = Manifest {
        app: "ServiceCompass",
        version: env!("CARGO_PKG_VERSION"),
        exported_at: Utc::now().to_rfc3339(),
        files: files
            .iter()
            .map(|(path, bytes)| ManifestFile {
                path: path.clone(),
                size: bytes.len() as u64,
                sha256: sha256(bytes),
            })
            .collect(),
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(anyhow::Error::from)?;

    let mut cursor = std::io::Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut cursor);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file("manifest.json", options)
        .map_err(anyhow::Error::from)?;
    zip.write_all(&manifest_bytes)
        .map_err(anyhow::Error::from)?;
    for (path, bytes) in files {
        zip.start_file(path, options).map_err(anyhow::Error::from)?;
        zip.write_all(&bytes).map_err(anyhow::Error::from)?;
    }
    zip.finish().map_err(anyhow::Error::from)?;
    Ok(cursor.into_inner())
}

fn read_required_file(path_in_zip: &str, path: PathBuf) -> AppResult<(String, Vec<u8>)> {
    let bytes = std::fs::read(&path).map_err(|error| {
        AppError::Internal(anyhow::anyhow!("读取 {} 失败：{}", path.display(), error))
    })?;
    Ok((path_in_zip.into(), bytes))
}

fn collect_icons(
    files: &mut Vec<(String, Vec<u8>)>,
    directory: PathBuf,
    prefix: &str,
) -> AppResult<()> {
    if !directory.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(&directory).map_err(anyhow::Error::from)? {
        let entry = entry.map_err(anyhow::Error::from)?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let zip_path = format!("{prefix}/{name}");
        if path.is_dir() {
            collect_icons(files, path, &zip_path)?;
        } else {
            files.push((zip_path, std::fs::read(path).map_err(anyhow::Error::from)?));
        }
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
