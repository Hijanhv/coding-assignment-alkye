use axum::{async_trait, extract::FromRequestParts, http::request::Parts};

use crate::{auth::Claims, error::AppError, state::AppState};

pub struct AuthUser(pub Claims);

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("missing Authorization header".to_string()))?;

        let token = header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AppError::Unauthorized("expected Bearer token".to_string()))?;

        let claims = crate::auth::verify_jwt(token, &state.config)?;
        Ok(AuthUser(claims))
    }
}
