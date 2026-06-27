use std::path::Path;

use anyhow::Result;
use chrono::Utc;
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};

use crate::auth;

pub const UNGROUPED_GROUP_ID: &str = "00000000-0000-0000-0000-000000000000";
pub const DEFAULT_SPACE_ID: &str = "00000000-0000-0000-0000-000000000001";

pub async fn connect(database_url: &str) -> Result<SqlitePool> {
    ensure_parent(database_url)?;
    let options = database_url
        .parse::<SqliteConnectOptions>()?
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePool::connect_with(options).await?;
    sqlx::migrate!().run(&pool).await?;
    ensure_defaults(&pool).await?;
    Ok(pool)
}

async fn ensure_defaults(pool: &SqlitePool) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT OR IGNORE INTO spaces (id, name, description, sort_order, created_at, updated_at) \
         VALUES (?, '默认空间', NULL, 0, ?, ?)",
    )
    .bind(DEFAULT_SPACE_ID)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT OR IGNORE INTO groups (id, space_id, name, description, sort_order, created_at, updated_at) \
         VALUES (?, ?, '未分组', NULL, -1, ?, ?)",
    )
    .bind(UNGROUPED_GROUP_ID)
    .bind(DEFAULT_SPACE_ID)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    sqlx::query("UPDATE groups SET space_id = ? WHERE space_id IS NULL OR space_id = ''")
        .bind(DEFAULT_SPACE_ID)
        .execute(pool)
        .await?;

    let has_admin: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM admin_users WHERE id = 1)")
            .fetch_one(pool)
            .await?;
    if !has_admin {
        let password_hash = auth::hash_password("admin")?;
        sqlx::query(
            "INSERT INTO admin_users (id, username, password_hash, updated_at) VALUES (1, 'admin', ?, ?)",
        )
        .bind(password_hash)
        .bind(now)
        .execute(pool)
        .await?;
        tracing::warn!("已创建默认管理员 admin，请登录后立即修改密码");
    }
    Ok(())
}

fn ensure_parent(database_url: &str) -> Result<()> {
    let Some(path) = database_url.strip_prefix("sqlite:") else {
        return Ok(());
    };
    if path == ":memory:" || path.contains("mode=memory") {
        return Ok(());
    }
    if let Some(parent) = Path::new(path.split('?').next().unwrap_or(path)).parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

pub async fn test_pool() -> SqlitePool {
    let options = SqliteConnectOptions::new()
        .filename(":memory:")
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .expect("无法创建测试数据库");
    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("测试数据库迁移失败");
    ensure_defaults(&pool)
        .await
        .expect("测试默认数据初始化失败");
    pool
}
