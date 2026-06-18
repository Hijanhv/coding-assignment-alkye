# AI Usage

## Tool used

Claude Code (claude-sonnet-4-6) via the Claude Code CLI.

## What the AI generated

The AI generated the initial structure and implementation for all source files, including:

- `Cargo.toml` — dependency selection and crate features
- `migrations/20240101000001_init.sql` — schema design (tables, PostgreSQL enum types)
- All `src/` modules: config, error, state, auth, cache, middleware, models, handlers
- `tests/integration.rs` — the full 11-test integration suite
- `.env.example`, `README.md`, `AI_USAGE.md`

## What was debugged and corrected manually (with AI assistance)

The following issues were found during the build/test cycle and fixed:

1. **Redis `ConnectionManager` missing feature flag** — the `connection-manager` feature was not initially included in the `redis` dependency. Added `"connection-manager"` to fix.

2. **Redis `set_ex` and `del` type inference** — Rust 1.95 (edition 2021) emits a future-incompatibility error around never-type fallback. Fixed by adding explicit type parameters: `set_ex::<_, _, ()>` and `del::<_, ()>`.

3. **SQLx compile-time query checking** — SQLx `query!` macros connect to the live database at compile time. The test database had a stale migration history from a prior project. Fixed by dropping and recreating both databases and restoring the schema before the build step.

4. **Unused `mut` on `State` destructuring** — two handler signatures had `State(mut state)` where `state` was not mutated. Removed the `mut`.

5. **Unused import warning in `handlers/auth.rs`** — the `Role` import was present for the `sqlx::query!` macro type annotation but the compiler treated it as unused. Accepted as a warning (the import is functionally needed by the macro).

6. **Test isolation** — initial tests were designed to run independently but could conflict when sharing the same database. Resolved by adding a `TRUNCATE ... CASCADE` at the start of each test and running with `--test-threads=1`.

## What was written by hand

The choice of which problems to tackle (e.g., dropping stale databases instead of trying to reconcile migration history, using `--test-threads=1` rather than per-test database isolation) was made by the developer reviewing the AI output and diagnosing the errors. The validation flow was manually executed with curl to confirm every response matches the assignment spec before recording the output in the README.
