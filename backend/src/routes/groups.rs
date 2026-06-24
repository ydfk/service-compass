use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    db::UNGROUPED_GROUP_ID,
    error::{AppError, AppResult},
    models::group::{Group, GroupInput, ReorderItem},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/groups", get(list).post(create))
        .route("/api/groups/reorder", post(reorder))
        .route("/api/groups/{id}", get(get_one).put(update).delete(remove))
}

async fn list(State(state): State<AppState>) -> AppResult<Json<Vec<Group>>> {
    let groups =
        sqlx::query_as::<_, Group>("SELECT * FROM groups WHERE id != ? ORDER BY sort_order, name")
            .bind(UNGROUPED_GROUP_ID)
            .fetch_all(&state.pool)
            .await?;
    Ok(Json(groups))
}

async fn get_one(State(state): State<AppState>, Path(id): Path<String>) -> AppResult<Json<Group>> {
    Ok(Json(find(&state, &id).await?))
}

async fn create(
    State(state): State<AppState>,
    Json(input): Json<GroupInput>,
) -> AppResult<Json<Group>> {
    validate(&input)?;
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO groups (id, name, description, icon, sort_order, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(input.name.trim())
    .bind(input.description)
    .bind(input.icon)
    .bind(input.sort_order)
    .bind(&now)
    .bind(&now)
    .execute(&state.pool)
    .await?;
    Ok(Json(find(&state, &id).await?))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<GroupInput>,
) -> AppResult<Json<Group>> {
    validate(&input)?;
    let result = sqlx::query(
        "UPDATE groups SET name = ?, description = ?, icon = ?, sort_order = ?, updated_at = ? \
         WHERE id = ?",
    )
    .bind(input.name.trim())
    .bind(input.description)
    .bind(input.icon)
    .bind(input.sort_order)
    .bind(Utc::now().to_rfc3339())
    .bind(&id)
    .execute(&state.pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(find(&state, &id).await?))
}

async fn remove(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let result = sqlx::query("DELETE FROM groups WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|error| {
            if error
                .as_database_error()
                .is_some_and(|value| value.is_foreign_key_violation())
            {
                AppError::Conflict
            } else {
                AppError::Database(error)
            }
        })?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn reorder(
    State(state): State<AppState>,
    Json(items): Json<Vec<ReorderItem>>,
) -> AppResult<Json<serde_json::Value>> {
    let mut transaction = state.pool.begin().await?;
    for item in items {
        sqlx::query("UPDATE groups SET sort_order = ?, updated_at = ? WHERE id = ?")
            .bind(item.sort_order)
            .bind(Utc::now().to_rfc3339())
            .bind(item.id)
            .execute(&mut *transaction)
            .await?;
    }
    transaction.commit().await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn find(state: &AppState, id: &str) -> AppResult<Group> {
    sqlx::query_as::<_, Group>("SELECT * FROM groups WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)
}

fn validate(input: &GroupInput) -> AppResult<()> {
    if input.name.trim().is_empty() {
        return Err(AppError::Validation("分组名称不能为空".into()));
    }
    Ok(())
}
