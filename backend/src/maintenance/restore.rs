use std::{
    fs,
    io::{Cursor, Read},
    path::{Component, Path, PathBuf},
};

use chrono::Utc;
use zip::ZipArchive;

use crate::{
    config::Config,
    error::{AppError, AppResult},
    icon::local,
    maintenance::archive,
};

pub fn apply_pending_restore(config: &Config) -> anyhow::Result<()> {
    let pending = pending_restore_dir(config);
    if !pending.exists() {
        return Ok(());
    }
    let db = pending.join("service-compass.db");
    let secret = pending.join("secret.key");
    if !db.exists() || !secret.exists() {
        anyhow::bail!("pending restore 缺少数据库或密钥文件");
    }

    let backup_dir = restore_backup_dir(config).join(format!(
        "restore-before-{}",
        Utc::now().format("%Y%m%d-%H%M%S")
    ));
    fs::create_dir_all(&backup_dir)?;
    backup_existing(config, &backup_dir)?;

    replace_file(
        &db,
        &archive::database_file(config).map_err(|error| anyhow::anyhow!(error))?,
    )?;
    replace_file(&secret, &config.secret_file)?;
    let target_icons = local::icon_directory(config);
    if target_icons.exists() {
        fs::remove_dir_all(&target_icons)?;
    }
    if pending.join("icons").exists() {
        copy_dir(&pending.join("icons"), &target_icons)?;
    }
    fs::remove_dir_all(pending)?;
    Ok(())
}

pub async fn prepare_pending_restore(config: &Config, bytes: Vec<u8>) -> AppResult<()> {
    let config = config.clone();
    tokio::task::spawn_blocking(move || extract_pending(&config, bytes))
        .await
        .map_err(anyhow::Error::from)??;
    Ok(())
}

fn extract_pending(config: &Config, bytes: Vec<u8>) -> AppResult<()> {
    let pending = pending_restore_dir(config);
    let temp = pending.with_extension("tmp");
    if temp.exists() {
        fs::remove_dir_all(&temp).map_err(anyhow::Error::from)?;
    }
    if pending.exists() {
        fs::remove_dir_all(&pending).map_err(anyhow::Error::from)?;
    }
    fs::create_dir_all(&temp).map_err(anyhow::Error::from)?;
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| AppError::Validation("ZIP 文件无效".into()))?;
    let mut has_manifest = false;
    let mut has_db = false;
    let mut has_secret = false;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(anyhow::Error::from)?;
        if file.is_dir() {
            continue;
        }
        let name = file.name().to_string();
        if !safe_zip_path(&name) {
            return Err(AppError::Validation("ZIP 包含非法路径".into()));
        }
        has_manifest |= name == "manifest.json";
        has_db |= name == "service-compass.db";
        has_secret |= name == "secret.key";
        if name != "manifest.json"
            && name != "service-compass.db"
            && name != "secret.key"
            && !name.starts_with("icons/")
        {
            continue;
        }
        let output = temp.join(&name);
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(anyhow::Error::from)?;
        }
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).map_err(anyhow::Error::from)?;
        fs::write(output, bytes).map_err(anyhow::Error::from)?;
    }
    if !has_manifest || !has_db || !has_secret {
        return Err(AppError::Validation(
            "导入包缺少 manifest、数据库或密钥文件".into(),
        ));
    }
    fs::rename(temp, pending).map_err(anyhow::Error::from)?;
    Ok(())
}

fn pending_restore_dir(config: &Config) -> PathBuf {
    data_dir(config).join("pending-restore")
}

fn restore_backup_dir(config: &Config) -> PathBuf {
    data_dir(config).join("restore-backups")
}

fn data_dir(config: &Config) -> PathBuf {
    config
        .secret_file
        .parent()
        .unwrap_or_else(|| Path::new("data"))
        .to_path_buf()
}

fn backup_existing(config: &Config, backup_dir: &Path) -> anyhow::Result<()> {
    let db = archive::database_file(config).map_err(|error| anyhow::anyhow!(error))?;
    if db.exists() {
        fs::copy(&db, backup_dir.join("service-compass.db"))?;
    }
    if config.secret_file.exists() {
        fs::copy(&config.secret_file, backup_dir.join("secret.key"))?;
    }
    let icons = local::icon_directory(config);
    if icons.exists() {
        copy_dir(&icons, &backup_dir.join("icons"))?;
    }
    Ok(())
}

fn replace_file(source: &Path, target: &Path) -> anyhow::Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, target)?;
    Ok(())
}

fn copy_dir(source: &Path, target: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir(&source_path, &target_path)?;
        } else {
            fs::copy(source_path, target_path)?;
        }
    }
    Ok(())
}

fn safe_zip_path(name: &str) -> bool {
    let path = Path::new(name);
    !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}
