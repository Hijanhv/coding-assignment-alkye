use redis::{aio::ConnectionManager, AsyncCommands};
use serde::{de::DeserializeOwned, Serialize};
use uuid::Uuid;

use crate::error::AppError;

pub const TASK_CACHE_TTL_SECS: u64 = 300;

pub fn task_cache_key(user_id: Uuid) -> String {
    format!("tasks:user:{}", user_id)
}

pub async fn get<T: DeserializeOwned>(
    redis: &mut ConnectionManager,
    key: &str,
) -> Result<Option<T>, AppError> {
    let raw: Option<String> = redis.get(key).await?;
    match raw {
        Some(s) => serde_json::from_str(&s)
            .map(Some)
            .map_err(|e| AppError::Internal(e.to_string())),
        None => Ok(None),
    }
}

pub async fn set<T: Serialize>(
    redis: &mut ConnectionManager,
    key: &str,
    value: &T,
    ttl: u64,
) -> Result<(), AppError> {
    let s = serde_json::to_string(value).map_err(|e| AppError::Internal(e.to_string()))?;
    redis.set_ex::<_, _, ()>(key, s, ttl).await?;
    Ok(())
}

pub async fn del(redis: &mut ConnectionManager, key: &str) -> Result<(), AppError> {
    redis.del::<_, ()>(key).await?;
    Ok(())
}
