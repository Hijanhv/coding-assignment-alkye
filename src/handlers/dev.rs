use axum::{extract::State, Json};

use crate::{error::AppError, models::email_log::EmailLog, state::AppState};

pub async fn latest_email_log(
    State(state): State<AppState>,
) -> Result<Json<EmailLog>, AppError> {
    let log = sqlx::query_as!(
        EmailLog,
        "SELECT id, recipient, subject, body, sent_at
         FROM email_logs
         ORDER BY sent_at DESC
         LIMIT 1"
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("no email logs found".to_string()))?;

    Ok(Json(log))
}
