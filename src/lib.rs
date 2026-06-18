pub mod auth;
pub mod cache;
pub mod config;
pub mod error;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod state;

use axum::{
    routing::{get, post},
    Router,
};
use redis::aio::ConnectionManager;
use sqlx::postgres::PgPoolOptions;
use tower_http::trace::TraceLayer;

use config::Config;
use state::AppState;

pub async fn build_app(config: Config) -> anyhow::Result<Router> {
    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!().run(&db).await?;

    let redis_client = redis::Client::open(config.redis_url.clone())?;
    let redis = ConnectionManager::new(redis_client).await?;

    let state = AppState { db, redis, config };

    let router = Router::new()
        .route("/seed/users", post(handlers::seed::seed_users))
        .route("/auth/login", post(handlers::auth::login))
        .route("/auth/verify-2fa", post(handlers::auth::verify_2fa))
        .route("/tasks", post(handlers::tasks::create_task))
        .route("/tasks/assign", post(handlers::tasks::assign_tasks))
        .route("/tasks/view-my-tasks", get(handlers::tasks::view_my_tasks))
        .route("/dev/email-logs/latest", get(handlers::dev::latest_email_log))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    Ok(router)
}
