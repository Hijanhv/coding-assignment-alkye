use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::net::SocketAddr;
use tokio::net::TcpListener;

const TEST_DATABASE_URL: &str = "postgresql://janhavi@localhost:5432/task_api_test";

async fn reset_and_spawn_server() -> (String, sqlx::PgPool) {
    dotenvy::dotenv().ok();

    let mut config = task_api::config::Config::from_env().expect("config");
    config.database_url = TEST_DATABASE_URL.to_string();

    // Build the app (runs migrations on first call)
    let app = task_api::build_app(config).await.expect("build app");

    // Connect directly to test DB for cleanup
    let pool = sqlx::PgPool::connect(TEST_DATABASE_URL).await.expect("connect");
    sqlx::query!("TRUNCATE TABLE email_logs, login_challenges, tasks, users RESTART IDENTITY CASCADE")
        .execute(&pool)
        .await
        .expect("truncate");

    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local addr");

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });

    (format!("http://{}", addr), pool)
}

async fn get_2fa_code(client: &Client, base: &str) -> String {
    let body: Value = client
        .get(format!("{}/dev/email-logs/latest", base))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    // body field contains "Your 2FA code is: 123456"
    body["body"]
        .as_str()
        .unwrap()
        .split(": ")
        .last()
        .unwrap()
        .trim()
        .to_string()
}

async fn login_and_get_token(client: &Client, base: &str, email: &str, password: &str) -> String {
    let challenge: Value = client
        .post(format!("{}/auth/login", base))
        .json(&json!({ "email": email, "password": password }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let challenge_id = challenge["login_challenge_id"].as_str().unwrap();
    let code = get_2fa_code(client, base).await;

    let token_resp: Value = client
        .post(format!("{}/auth/verify-2fa", base))
        .json(&json!({ "login_challenge_id": challenge_id, "code": code }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    token_resp["access_token"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn test_seed_creates_users() {
    let (base, _pool) = reset_and_spawn_server().await;
    let client = Client::new();

    let resp = client
        .post(format!("{}/seed/users", base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["admin"]["email"], "admin@example.com");
    assert_eq!(body["admin"]["role"], "admin");
    assert_eq!(body["james_bond"]["email"], "jamesbond@example.com");
    assert_eq!(body["james_bond"]["role"], "staff");
}

#[tokio::test]
async fn test_seed_is_idempotent() {
    let (base, _pool) = reset_and_spawn_server().await;
    let client = Client::new();

    let r1: Value = client
        .post(format!("{}/seed/users", base))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let r2: Value = client
        .post(format!("{}/seed/users", base))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(r1["admin"]["id"], r2["admin"]["id"], "seed must be idempotent");
}

#[tokio::test]
async fn test_login_returns_challenge_not_token() {
    let (base, _pool) = reset_and_spawn_server().await;
    let client = Client::new();

    client
        .post(format!("{}/seed/users", base))
        .send()
        .await
        .unwrap();

    let resp = client
        .post(format!("{}/auth/login", base))
        .json(&json!({ "email": "admin@example.com", "password": "AdminPass123!" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert!(
        body["login_challenge_id"].is_string(),
        "login must return a challenge id, not a token"
    );
    assert!(
        body.get("access_token").is_none(),
        "login must NOT return an access_token"
    );
}

#[tokio::test]
async fn test_wrong_password_rejected() {
    let (base, _pool) = reset_and_spawn_server().await;
    let client = Client::new();

    client
        .post(format!("{}/seed/users", base))
        .send()
        .await
        .unwrap();

    let resp = client
        .post(format!("{}/auth/login", base))
        .json(&json!({ "email": "admin@example.com", "password": "wrongpassword" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_wrong_2fa_code_rejected() {
    let (base, _pool) = reset_and_spawn_server().await;
    let client = Client::new();

    client
        .post(format!("{}/seed/users", base))
        .send()
        .await
        .unwrap();

    let resp: Value = client
        .post(format!("{}/auth/login", base))
        .json(&json!({ "email": "admin@example.com", "password": "AdminPass123!" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let challenge_id = resp["login_challenge_id"].as_str().unwrap();

    let status = client
        .post(format!("{}/auth/verify-2fa", base))
        .json(&json!({ "login_challenge_id": challenge_id, "code": "000000" }))
        .send()
        .await
        .unwrap()
        .status();

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_reused_2fa_code_rejected() {
    let (base, _pool) = reset_and_spawn_server().await;
    let client = Client::new();

    client
        .post(format!("{}/seed/users", base))
        .send()
        .await
        .unwrap();

    let resp: Value = client
        .post(format!("{}/auth/login", base))
        .json(&json!({ "email": "admin@example.com", "password": "AdminPass123!" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let challenge_id = resp["login_challenge_id"].as_str().unwrap().to_string();
    let code = get_2fa_code(&client, &base).await;

    // First verification — OK
    let s1 = client
        .post(format!("{}/auth/verify-2fa", base))
        .json(&json!({ "login_challenge_id": challenge_id, "code": code }))
        .send()
        .await
        .unwrap()
        .status();
    assert_eq!(s1, StatusCode::OK);

    // Second verification with same code — rejected
    let s2 = client
        .post(format!("{}/auth/verify-2fa", base))
        .json(&json!({ "login_challenge_id": challenge_id, "code": code }))
        .send()
        .await
        .unwrap()
        .status();
    assert_eq!(s2, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_unauthenticated_request_rejected() {
    let (base, _pool) = reset_and_spawn_server().await;
    let client = Client::new();

    let status = client
        .get(format!("{}/tasks/view-my-tasks", base))
        .send()
        .await
        .unwrap()
        .status();

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_staff_cannot_create_task() {
    let (base, _pool) = reset_and_spawn_server().await;
    let client = Client::new();

    client
        .post(format!("{}/seed/users", base))
        .send()
        .await
        .unwrap();

    let token = login_and_get_token(&client, &base, "jamesbond@example.com", "Bond007!").await;

    let status = client
        .post(format!("{}/tasks", base))
        .bearer_auth(&token)
        .json(&json!({ "title": "Sneaky task", "priority": "low" }))
        .send()
        .await
        .unwrap()
        .status();

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_admin_can_create_task() {
    let (base, _pool) = reset_and_spawn_server().await;
    let client = Client::new();

    client
        .post(format!("{}/seed/users", base))
        .send()
        .await
        .unwrap();

    let token = login_and_get_token(&client, &base, "admin@example.com", "AdminPass123!").await;

    let resp = client
        .post(format!("{}/tasks", base))
        .bearer_auth(&token)
        .json(&json!({ "title": "Admin task", "priority": "high" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["title"], "Admin task");
    assert_eq!(body["status"], "todo");
    assert_eq!(body["priority"], "high");
}

#[tokio::test]
async fn test_full_workflow() {
    let (base, _pool) = reset_and_spawn_server().await;
    let client = Client::new();

    // Seed users
    let seed: Value = client
        .post(format!("{}/seed/users", base))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let james_id = seed["james_bond"]["id"].as_str().unwrap().to_string();

    // Admin logs in
    let admin_token = login_and_get_token(&client, &base, "admin@example.com", "AdminPass123!").await;

    // Create exactly 5 tasks with the 3 priorities we'll verify in the response
    let task_configs = [
        ("Mission Briefing", "high"),
        ("Gadget Procurement", "medium"),
        ("Field Report", "low"),
        ("Background Check", "medium"),
        ("Debrief Session", "high"),
    ];
    let mut task_ids: Vec<String> = Vec::new();

    for (title, priority) in &task_configs {
        let body: Value = client
            .post(format!("{}/tasks", base))
            .bearer_auth(&admin_token)
            .json(&json!({ "title": title, "priority": priority }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        task_ids.push(body["id"].as_str().unwrap().to_string());
    }

    assert_eq!(task_ids.len(), 5, "exactly 5 tasks must be created");

    // Assign first 3 to James Bond (high, medium, low priorities)
    let assign_body: Value = client
        .post(format!("{}/tasks/assign", base))
        .bearer_auth(&admin_token)
        .json(&json!({ "task_ids": &task_ids[..3], "user_id": james_id }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(assign_body["assigned"], 3);

    // James Bond logs in
    let jb_token = login_and_get_token(&client, &base, "jamesbond@example.com", "Bond007!").await;

    // James Bond cannot create tasks — 403
    let forbidden = client
        .post(format!("{}/tasks", base))
        .bearer_auth(&jb_token)
        .json(&json!({ "title": "Not allowed", "priority": "low" }))
        .send()
        .await
        .unwrap()
        .status();
    assert_eq!(forbidden, StatusCode::FORBIDDEN);

    // First GET /tasks/view-my-tasks — DB hit, cache.hit = false
    let resp1: Value = client
        .get(format!("{}/tasks/view-my-tasks", base))
        .bearer_auth(&jb_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(resp1["user"]["email"], "jamesbond@example.com");
    assert_eq!(resp1["user"]["role"], "staff");
    assert_eq!(
        resp1["tasks"].as_array().unwrap().len(),
        3,
        "James Bond must see exactly 3 tasks"
    );
    assert_eq!(resp1["summary"]["total_assigned_tasks"], 3);
    assert_eq!(resp1["cache"]["hit"], false, "first call must not be from cache");

    for task in resp1["tasks"].as_array().unwrap() {
        assert_eq!(task["assigned_to"], "jamesbond@example.com");
        assert_eq!(task["status"], "todo");
        assert!(
            ["high", "medium", "low"].contains(&task["priority"].as_str().unwrap()),
            "priority must be high, medium, or low"
        );
    }

    // Second GET /tasks/view-my-tasks — cache hit
    let resp2: Value = client
        .get(format!("{}/tasks/view-my-tasks", base))
        .bearer_auth(&jb_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(resp2["cache"]["hit"], true, "second call must come from cache");
    assert_eq!(resp2["tasks"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn test_cache_invalidates_on_reassignment() {
    let (base, _pool) = reset_and_spawn_server().await;
    let client = Client::new();

    let seed: Value = client
        .post(format!("{}/seed/users", base))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let james_id = seed["james_bond"]["id"].as_str().unwrap().to_string();

    let admin_token = login_and_get_token(&client, &base, "admin@example.com", "AdminPass123!").await;
    let jb_token = login_and_get_token(&client, &base, "jamesbond@example.com", "Bond007!").await;

    // Create a task and assign to James Bond
    let task: Value = client
        .post(format!("{}/tasks", base))
        .bearer_auth(&admin_token)
        .json(&json!({ "title": "Cache Test Task", "priority": "medium" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let task_id = task["id"].as_str().unwrap();

    client
        .post(format!("{}/tasks/assign", base))
        .bearer_auth(&admin_token)
        .json(&json!({ "task_ids": [task_id], "user_id": james_id }))
        .send()
        .await
        .unwrap();

    // Warm the cache — first call is a DB hit
    let r1: Value = client
        .get(format!("{}/tasks/view-my-tasks", base))
        .bearer_auth(&jb_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(r1["cache"]["hit"], false);

    // Second call hits cache
    let r2: Value = client
        .get(format!("{}/tasks/view-my-tasks", base))
        .bearer_auth(&jb_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(r2["cache"]["hit"], true);

    // Re-assign the same task (triggers cache invalidation)
    client
        .post(format!("{}/tasks/assign", base))
        .bearer_auth(&admin_token)
        .json(&json!({ "task_ids": [task_id], "user_id": james_id }))
        .send()
        .await
        .unwrap();

    // After invalidation, next call must be a fresh DB hit
    let r3: Value = client
        .get(format!("{}/tasks/view-my-tasks", base))
        .bearer_auth(&jb_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        r3["cache"]["hit"], false,
        "cache must be invalidated after assignment"
    );
}
