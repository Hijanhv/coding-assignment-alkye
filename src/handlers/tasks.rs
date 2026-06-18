use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    cache,
    error::AppError,
    middleware::AuthUser,
    models::{task::TaskPriority, user::Role},
    state::AppState,
};

#[derive(Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub priority: TaskPriority,
}

#[derive(Serialize, Deserialize)]
pub struct TaskItem {
    pub id: Uuid,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub assigned_to: Option<String>,
}

pub async fn create_task(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<CreateTaskRequest>,
) -> Result<Json<TaskItem>, AppError> {
    if claims.role != Role::Admin {
        return Err(AppError::Forbidden(
            "only admins can create tasks".to_string(),
        ));
    }

    let row = sqlx::query!(
        r#"INSERT INTO tasks (title, description, priority, created_by_id)
           VALUES ($1, $2, $3, $4)
           RETURNING id, title, status::text, priority::text"#,
        body.title,
        body.description,
        body.priority as TaskPriority,
        claims.sub,
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(TaskItem {
        id: row.id,
        title: row.title,
        status: row.status.unwrap_or_default(),
        priority: row.priority.unwrap_or_default(),
        assigned_to: None,
    }))
}

#[derive(Deserialize)]
pub struct AssignTasksRequest {
    pub task_ids: Vec<Uuid>,
    pub user_id: Uuid,
}

#[derive(Serialize)]
pub struct AssignResponse {
    pub assigned: u64,
}

pub async fn assign_tasks(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<AssignTasksRequest>,
) -> Result<Json<AssignResponse>, AppError> {
    if claims.role != Role::Admin {
        return Err(AppError::Forbidden(
            "only admins can assign tasks".to_string(),
        ));
    }

    let result = sqlx::query!(
        "UPDATE tasks SET assigned_to_id = $1, updated_at = NOW()
         WHERE id = ANY($2)",
        body.user_id,
        &body.task_ids,
    )
    .execute(&state.db)
    .await?;

    let key = cache::task_cache_key(body.user_id);
    let mut redis = state.redis.clone();
    cache::del(&mut redis, &key).await?;

    Ok(Json(AssignResponse {
        assigned: result.rows_affected(),
    }))
}

#[derive(Serialize, Deserialize)]
struct CachedTasks {
    tasks: Vec<TaskItem>,
}

#[derive(Serialize)]
pub struct ViewMyTasksResponse {
    pub user: UserSummary,
    pub tasks: Vec<TaskItem>,
    pub summary: Summary,
    pub cache: CacheMeta,
}

#[derive(Serialize)]
pub struct UserSummary {
    pub email: String,
    pub role: String,
}

#[derive(Serialize)]
pub struct Summary {
    pub total_assigned_tasks: usize,
}

#[derive(Serialize)]
pub struct CacheMeta {
    pub hit: bool,
}

pub async fn view_my_tasks(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<ViewMyTasksResponse>, AppError> {
    let cache_key = cache::task_cache_key(claims.sub);
    let mut redis = state.redis.clone();

    if let Some(cached) = cache::get::<CachedTasks>(&mut redis, &cache_key).await? {
        let total = cached.tasks.len();
        return Ok(Json(ViewMyTasksResponse {
            user: UserSummary {
                email: claims.email,
                role: role_str(&claims.role),
            },
            tasks: cached.tasks,
            summary: Summary {
                total_assigned_tasks: total,
            },
            cache: CacheMeta { hit: true },
        }));
    }

    let rows = sqlx::query!(
        r#"SELECT t.id, t.title, t.status::text AS status, t.priority::text AS priority,
                  u.email AS assigned_to
           FROM tasks t
           JOIN users u ON t.assigned_to_id = u.id
           WHERE t.assigned_to_id = $1
           ORDER BY t.created_at ASC"#,
        claims.sub,
    )
    .fetch_all(&state.db)
    .await?;

    let tasks: Vec<TaskItem> = rows
        .into_iter()
        .map(|r| TaskItem {
            id: r.id,
            title: r.title,
            status: r.status.unwrap_or_default(),
            priority: r.priority.unwrap_or_default(),
            assigned_to: Some(r.assigned_to),
        })
        .collect();

    let payload = CachedTasks {
        tasks: tasks
            .iter()
            .map(|t| TaskItem {
                id: t.id,
                title: t.title.clone(),
                status: t.status.clone(),
                priority: t.priority.clone(),
                assigned_to: t.assigned_to.clone(),
            })
            .collect(),
    };
    cache::set(&mut redis, &cache_key, &payload, cache::TASK_CACHE_TTL_SECS).await?;

    let total = tasks.len();
    Ok(Json(ViewMyTasksResponse {
        user: UserSummary {
            email: claims.email,
            role: role_str(&claims.role),
        },
        tasks,
        summary: Summary {
            total_assigned_tasks: total,
        },
        cache: CacheMeta { hit: false },
    }))
}

fn role_str(role: &Role) -> String {
    match role {
        Role::Admin => "admin".to_string(),
        Role::Staff => "staff".to_string(),
    }
}
