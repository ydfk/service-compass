use std::{env, path::PathBuf};

use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct Config {
    pub bind: String,
    pub database_url: String,
    pub secret_key: Option<String>,
    pub secret_file: PathBuf,
    pub static_dir: PathBuf,
    pub log_dir: PathBuf,
    pub log_retention_days: i64,
    pub production: bool,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let production = env_bool("SERVICECOMPASS_PRODUCTION", false)?;
        Ok(Self {
            bind: env::var("SERVICECOMPASS_BIND").unwrap_or_else(|_| "127.0.0.1:3000".into()),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:data/service-compass.db".into()),
            secret_key: env::var("SERVICECOMPASS_SECRET_KEY").ok(),
            secret_file: env::var("SERVICECOMPASS_SECRET_FILE")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    if production {
                        PathBuf::from("/data/secret.key")
                    } else {
                        PathBuf::from("data/secret.key")
                    }
                }),
            static_dir: env::var("SERVICECOMPASS_STATIC_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("frontend/dist")),
            log_dir: env::var("SERVICECOMPASS_LOG_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    if production {
                        PathBuf::from("/data/logs")
                    } else {
                        PathBuf::from("data/logs")
                    }
                }),
            log_retention_days: env_i64("SERVICECOMPASS_LOG_RETENTION_DAYS", 30)?,
            production,
        })
    }
}

fn env_bool(name: &str, default: bool) -> Result<bool> {
    let Ok(value) = env::var(name) else {
        return Ok(default);
    };
    value
        .parse::<bool>()
        .with_context(|| format!("{name} 必须是 true 或 false"))
}

fn env_i64(name: &str, default: i64) -> Result<i64> {
    let Ok(value) = env::var(name) else {
        return Ok(default);
    };
    value
        .parse::<i64>()
        .with_context(|| format!("{name} 必须是整数"))
}
