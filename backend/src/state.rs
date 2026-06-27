use std::{collections::HashSet, sync::Arc};

use sqlx::SqlitePool;
use tokio::sync::{RwLock, broadcast};

use crate::{config::Config, crypto::SecretBox};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Arc<Config>,
    pub sessions: Arc<RwLock<HashSet<String>>>,
    pub secrets: Arc<SecretBox>,
    pub dashboard_events: broadcast::Sender<String>,
}

impl AppState {
    pub fn new(pool: SqlitePool, config: Arc<Config>) -> anyhow::Result<Self> {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let secrets = Arc::new(SecretBox::load(&config)?);
        Ok(Self {
            pool,
            config,
            sessions: Arc::new(RwLock::new(HashSet::new())),
            secrets,
            dashboard_events: broadcast::channel(64).0,
        })
    }
}
