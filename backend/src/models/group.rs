use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Clone, Debug, FromRow, Serialize)]
pub struct Space {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize)]
pub struct SpaceInput {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub sort_order: i64,
}

#[derive(Clone, Debug, FromRow, Serialize)]
pub struct Group {
    pub id: String,
    pub space_id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize)]
pub struct GroupInput {
    pub space_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    #[serde(default)]
    pub sort_order: i64,
}

#[derive(Deserialize)]
pub struct ReorderItem {
    pub id: String,
    pub sort_order: i64,
}
