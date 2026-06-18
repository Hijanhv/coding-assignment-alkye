use axum::{extract::State, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::{create_jwt, generate_otp, hash_otp, verify_password},
    error::AppError,
    models::user::Role,
    state::AppState,
};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub login_challenge_id: Uuid,
}

pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let user = sqlx::query!(
        r#"SELECT id, hashed_password FROM users WHERE email = $1"#,
        body.email
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::Unauthorized("invalid credentials".to_string()))?;

    if !verify_password(&body.password, &user.hashed_password)? {
        return Err(AppError::Unauthorized("invalid credentials".to_string()));
    }

    let code = generate_otp();
    let code_hash = hash_otp(&code);
    let expires_at = Utc::now() + chrono::Duration::minutes(5);

    let challenge = sqlx::query!(
        "INSERT INTO login_challenges (user_id, code_hash, expires_at)
         VALUES ($1, $2, $3)
         RETURNING id",
        user.id,
        code_hash,
        expires_at,
    )
    .fetch_one(&state.db)
    .await?;

    sqlx::query!(
        "INSERT INTO email_logs (recipient, subject, body)
         VALUES ($1, $2, $3)",
        body.email,
        "Your verification code",
        format!("Your 2FA code is: {}", code),
    )
    .execute(&state.db)
    .await?;

    tracing::info!(email = %body.email, code = %code, "2FA code generated");

    Ok(Json(LoginResponse {
        login_challenge_id: challenge.id,
    }))
}

#[derive(Deserialize)]
pub struct Verify2faRequest {
    pub login_challenge_id: Uuid,
    pub code: String,
}

#[derive(Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
}

pub async fn verify_2fa(
    State(state): State<AppState>,
    Json(body): Json<Verify2faRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    let challenge = sqlx::query!(
        r#"SELECT id, user_id, code_hash, expires_at, used
           FROM login_challenges
           WHERE id = $1"#,
        body.login_challenge_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::BadRequest("challenge not found".to_string()))?;

    if challenge.used {
        return Err(AppError::BadRequest("code already used".to_string()));
    }

    if Utc::now() > challenge.expires_at {
        return Err(AppError::BadRequest("code expired".to_string()));
    }

    if hash_otp(&body.code) != challenge.code_hash {
        return Err(AppError::Unauthorized("invalid code".to_string()));
    }

    sqlx::query!(
        "UPDATE login_challenges SET used = TRUE WHERE id = $1",
        challenge.id
    )
    .execute(&state.db)
    .await?;

    let user = sqlx::query!(
        r#"SELECT id, email, role AS "role: Role" FROM users WHERE id = $1"#,
        challenge.user_id,
    )
    .fetch_one(&state.db)
    .await?;

    let token = create_jwt(user.id, &user.email, &user.role, &state.config)?;

    Ok(Json(TokenResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
    }))
}
