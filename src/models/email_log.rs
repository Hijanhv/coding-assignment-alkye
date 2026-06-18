use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct EmailLog {
    pub id: Uuid,
    pub recipient: String,
    pub subject: String,
    pub body: String,
    pub sent_at: DateTime<Utc>,
}
