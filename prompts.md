# Prompts

# Rust Backend Developer Coding Assignment
## Task Management API with Auth, 2FA, Role-Based Access, and Caching

**Time expectation:** 45 min
**Primary stack:** Rust web API
**Validation point:** `GET /tasks/view-my-tasks`

---

## Objective

Build a Rust backend API for a simple task management workflow. The goal is to test authentication, two-factor login, role-based permissions, task assignment, caching, and clean backend structure.

AI tools are allowed. We do not mind how the candidate builds it, but they must understand and explain the submitted code.

---

## Required Workflow

When the API is running locally, we should be able to complete this flow using curl, Postman, or Swagger/OpenAPI documentation.

1. Create two users: **Admin** and **James Bond**.
2. Start login as Admin using email and password. The API should trigger a two-factor email code and return a `login_challenge_id`, **not** a JWT.
3. Retrieve the verification code from a local development email log or console output.
4. Verify Admin 2FA and receive an Admin JWT token.
5. Create exactly **5 tasks** as Admin.
6. Assign exactly **3** of those tasks to James Bond.
7. Start login as James Bond and retrieve his two-factor verification code.
8. Verify James Bond 2FA and receive a James Bond JWT token.
9. Attempt to create a task as James Bond. This must return **403 Forbidden**.
10. Call `GET /tasks/view-my-tasks` as James Bond. It must return exactly **3 assigned tasks**.
11. Call `GET /tasks/view-my-tasks` again. The response should come from cache and show `cache.hit = true`.

---

## Expected API Shape

The exact route names can vary slightly, but the following capabilities must exist.

| Endpoint | Purpose |
|---|---|
| `POST /seed/users` | Create Admin and James Bond users for validation. |
| `POST /auth/login` | Validate email/password, create a 2FA challenge, and trigger an email code. |
| `GET /dev/email-logs/latest` | Development-only endpoint to view the latest sent verification code. |
| `POST /auth/verify-2fa` | Verify the code and return a JWT access token. |
| `POST /tasks` | Create a task. Admin only. |
| `POST /tasks/assign` | Assign selected tasks to James Bond. Admin only. |
| `GET /tasks/view-my-tasks` | Return tasks assigned to the logged-in user, with cache metadata. |

---

## Final Validation Response

The main validation point is the final James Bond response. If this response is correct, we can quickly confirm that the core assignment works.

```
GET /tasks/view-my-tasks
Authorization: Bearer JAMES_BOND_TOKEN
```

**Expected response:**

```json
{
  "user": {
    "email": "jamesbond@example.com",
    "role": "staff"
  },
  "tasks": [
    {
      "id": "...",
      "title": "...",
      "status": "todo",
      "priority": "high",
      "assigned_to": "jamesbond@example.com"
    },
    {
      "id": "...",
      "title": "...",
      "status": "todo",
      "priority": "medium",
      "assigned_to": "jamesbond@example.com"
    },
    {
      "id": "...",
      "title": "...",
      "status": "todo",
      "priority": "low",
      "assigned_to": "jamesbond@example.com"
    }
  ],
  "summary": {
    "total_assigned_tasks": 3
  },
  "cache": {
    "hit": false
  }
}
```

Calling the same endpoint again should return `cache.hit = true`.

---

## Minimum Data Model

Candidates may design the data model however they prefer, but the implementation should support at least these entities:

- **User**
- **Task**
- **LoginChallenge** or **TwoFactorChallenge**
- **EmailLog** or equivalent development email record

A **User** should include: `id`, `full_name`, `email`, `hashed_password`, `role`, `created_at`, `updated_at`.

A **Task** should include: `id`, `title`, `description`, `status`, `priority`, `created_by_id`, `assigned_to_id`, `created_at`, `updated_at`.

---

## Business Rules

- Users have roles: `admin` or `staff`.
- Only Admin can create tasks.
- Only Admin can assign tasks.
- James Bond must not be able to create tasks.
- James Bond must only see tasks assigned to him.
- Exactly 5 tasks must be created during the validation flow.
- Exactly 3 tasks must be assigned to James Bond.
- The `view-my-tasks` response must be generated from the database, not hardcoded.

---

## Two-Factor Authentication Requirement

- Login must use email-based two-factor authentication. The first login request should not return a JWT immediately. It should create a challenge, generate a one-time code, and trigger an email event.
- Verification codes expire after 5 minutes.
- Verification codes can only be used once.
- Incorrect and expired codes must be rejected.
- A JWT is issued only after successful verification.
- Real email delivery is optional. SMTP, Mailtrap, console logging, or an `email_logs` table are acceptable.
- If persisted, the verification code should not be stored in plain text.

---

## Caching Requirement

- The `GET /tasks/view-my-tasks` endpoint must use caching per user.
- The first request should load from the database and return `cache.hit = false`.
- The second identical request should return from cache and return `cache.hit = true`.
- When tasks are assigned or updated, the affected user's task cache must be invalidated.
- Redis is preferred. In-memory cache is acceptable if the limitation is documented.

---

## Rust Technical Requirements

Use Rust and a production-appropriate web API stack. Suggested options:

- Rust stable, edition 2021 or later
- Axum or Actix Web
- SQLx, SeaORM, or Diesel
- PostgreSQL preferred; SQLite acceptable for local development
- JWT authentication
- Argon2 or bcrypt password hashing
- Serde for request/response models
- Tokio async runtime
- Migrations using sqlx-cli, refinery, Diesel migrations, or equivalent
- Tests using `cargo test` plus integration tests where appropriate

**Optional:** Docker Compose, Redis, OpenAPI generation, tracing/logging, rustfmt, clippy, GitHub Actions.

---

## Testing Expectations

- Admin and James Bond can be created.
- Login creates a 2FA challenge and does not immediately return a JWT.
- Correct 2FA code returns a JWT.
- Incorrect, expired, or reused 2FA codes are rejected.
- Admin can create 5 tasks.
- Admin can assign exactly 3 tasks to James Bond.
- James Bond cannot create a task.
- James Bond can view exactly 3 assigned tasks.
- Calling `view-my-tasks` twice shows `cache.hit` false, then true.
- Task assignment/update invalidates the affected cache.

---

## Submission Requirements

- GitHub repository link
- `README.md` with setup, migration, run, seed, validation, and test instructions
- `AI_USAGE.md` explaining which AI tools were used and what was manually changed
- `.env.example`
- Application code, migrations, and tests
- The final `GET /tasks/view-my-tasks` response pasted into the README

---

## Assumptions Candidates Can Make

- This is a local development assignment, not a deployed production service.
- A simple seed endpoint or seed script is acceptable for creating Admin and James Bond.
- Real email delivery is not required if email events can be validated locally.
- In-memory caching is acceptable if Redis is not used, but it must be documented.
- Task statuses and priorities can be simple enums or constrained strings.
- No frontend is required.
- The API can use UUIDs or integer IDs.
- The exact folder structure is up to the candidate, as long as it is clean and maintainable.
- It is acceptable to make small route-name changes if the README clearly documents the validation workflow.

---

## Evaluation Criteria

- The required workflow works end to end.
- Authentication, 2FA, and JWT handling are implemented correctly.
- Role-based permissions are enforced.
- Caching works and invalidates correctly.
- James Bond sees exactly 3 assigned tasks and cannot create tasks.
- The code is idiomatic, maintainable Rust.
- Tests cover the core workflow.
- The candidate can explain their design and AI usage.

---

## Additional Notes

- Use Redis for caching — no need to deploy this on Vercel; build only what's mentioned in the assignment JD.
- No AI-generated boilerplate/slop in the code.
- Once done, check for errors and bugs, run the tests, take a screenshot of the tests passing, and add it to the README.
- In the README, document everything that was done.
```

