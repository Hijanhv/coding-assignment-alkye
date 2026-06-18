use axum::{extract::State, Json};
use serde::Serialize;
use uuid::Uuid;

use crate::{auth::hash_password, error::AppError, models::user::Role, state::AppState};

#[derive(Serialize)]
pub struct SeedResponse {
    pub admin: SeedUser,
    pub james_bond: SeedUser,
}

#[derive(Serialize)]
pub struct SeedUser {
    pub id: Uuid,
    pub email: String,
    pub role: Role,
}

pub async fn seed_users(State(state): State<AppState>) -> Result<Json<SeedResponse>, AppError> {
    let admin = upsert_user(
        &state,
        "Admin",
        "admin@example.com",
        "AdminPass123!",
        Role::Admin,
    )
    .await?;

    let james_bond = upsert_user(
        &state,
        "James Bond",
        "jamesbond@example.com",
        "Bond007!",
        Role::Staff,
    )
    .await?;

    Ok(Json(SeedResponse { admin, james_bond }))
}

async fn upsert_user(
    state: &AppState,
    full_name: &str,
    email: &str,
    password: &str,
    role: Role,
) -> Result<SeedUser, AppError> {
    if let Some(row) = sqlx::query!(
        "SELECT id, email, role AS \"role: Role\" FROM users WHERE email = $1",
        email
    )
    .fetch_optional(&state.db)
    .await?
    {
        return Ok(SeedUser {
            id: row.id,
            email: row.email,
            role: row.role,
        });
    }

    let hashed = hash_password(password)?;
    let row = sqlx::query!(
        r#"INSERT INTO users (full_name, email, hashed_password, role)
           VALUES ($1, $2, $3, $4)
           RETURNING id, email, role AS "role: Role""#,
        full_name,
        email,
        hashed,
        role as Role,
    )
    .fetch_one(&state.db)
    .await?;

    Ok(SeedUser {
        id: row.id,
        email: row.email,
        role: row.role,
    })
}
