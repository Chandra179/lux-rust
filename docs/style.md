# Rust style guide

Style rules for the Rust code in this repo (`server/` and any crate added to the workspace).
The frontend is covered by `make web-lint` and `make web-typecheck`.

Every rule is tagged by who enforces it:

- **tool**: a command fails CI when the rule is broken.
- **review**: nothing checks it automatically; reviewers do.

When a rule here and a tool disagree, fix the tool config or this file in the same PR.

## 1. Tooling is the baseline

| Rule | Enforced by |
|---|---|
| Formatting is `rustfmt` with the Rust Style Guide defaults. `rustfmt.toml` only sets `edition`, `newline_style` and `use_field_init_shorthand`; do not add other style options. | **tool** `cargo fmt --all --check` (`make fmt` rewrites) |
| `clippy` runs on all targets with warnings denied. | **tool** `make lint` = `cargo clippy --workspace --all-targets -- -D warnings` |
| The lint set is declared once in `[workspace.lints]` in the root `Cargo.toml`, and each crate opts in with `[lints] workspace = true`. Add or relax a lint there, not with crate-level attributes. There is no `clippy.toml`. | **tool** clippy (above) |
| Today's set: `unsafe_code = forbid`, `missing_debug_implementations`, `missing_docs`, clippy `all`, `unwrap_used`, `expect_used`, `dbg_macro`, `todo` and `unimplemented` (all `warn`, so `-D warnings` makes them errors). `Cargo.toml` is the source of truth. | **tool** clippy / rustc |
| Edition 2024 and Rust 1.99.1 are pinned: the edition and `rust-version` in `[workspace.package]`, the toolchain (with `rustfmt` and `clippy`) in `rust-toolchain.toml`. Raising either is its own PR. | **tool** rustup selects the pinned toolchain in this repo |
| Override a lint locally with `#[expect(lint, reason = "...")]`, not `#[allow]`; a stale `expect` then warns on its own. The two test-code exemptions in section 3 are the only crate-level `allow`s. | **review** |
| Dependency policy (advisories, licences, duplicates) is checked in CI. | **tool** `make verify` = `cargo deny check` |

## 2. Naming and API shape

- Casing follows RFC 430: `UpperCamelCase` for types, traits and enum variants; `snake_case` for
  functions, methods, modules and locals; `SCREAMING_SNAKE_CASE` for constants and statics.
  **tool** rustc's `non_*_case` lints (warn by default, denied by `-D warnings`).
- Acronyms count as one word: `HttpClient`, `RequestId`, `Uuid`; in `snake_case` they are
  lowercase (`request_id`). **tool** clippy `upper_case_acronyms` catches all-caps names; the rest is
  **review**.
- Conversions are named by cost and ownership: `as_*` borrows for free, `to_*` does work or
  copies, `into_*` consumes `self`. **tool** clippy `wrong_self_convention` checks the `self` kind;
  choosing the right prefix is **review**.
- Getters drop the `get_` prefix: `name()` and `name_mut()`. Keep `get` for the single obvious value
  (`Cell::get`). **review**
- Constructors: `new` for the primary one, `with_*` for variants, `from_*` for conversions. If a
  type has a no-argument `new`, it also implements `Default`. Prefer implementing `From`, `TryFrom`
  or `FromStr` over a hand-named `from_*` when the conversion is the point. **tool** clippy
  `new_without_default`; the rest is **review**.
- Every public type implements `Debug`. **tool** `missing_debug_implementations`. A type that holds a
  secret or a handle (`KvStore` in `store/kv.rs`) implements it by hand with `finish_non_exhaustive()`
  so nothing sensitive is printed. **review**
- Take `&str` / `&[T]` in parameters, not `&String` / `&Vec<T>`. **tool** clippy `ptr_arg`
- JSON object keys are `snake_case` (serde's default; `Example` serialises as `id`, `name`,
  `created_at`). Do not switch them to `PascalCase` or `camelCase` with `#[serde(rename_all)]`; the
  React client mirrors the Rust names. **review**

## 3. Errors

- **Domain errors** are a `thiserror` enum per module, in `modules/<name>/error.rs`. Variants name
  the failure (`ReservedName`, `Store`), not the layer that raised it, and wrap the underlying error
  with `#[from]` so it stays the `source()`. **review**
- **`anyhow` is only for the binary edge**: `main`, `run()` and startup code. Modules never return
  `anyhow::Error`. **review**
- **No `unwrap()` or `expect()` outside tests and startup.** `unwrap_used` and `expect_used` are
  `warn` in `[workspace.lints]`, so `-D warnings` fails the build. **tool** Startup code does not
  need them: it returns errors with `?`. Test code is exempt through exactly two attributes:
  `#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]` in `lib.rs` (unit tests) and
  `#![allow(clippy::unwrap_used, clippy::expect_used)]` at the top of `tests/api/main.rs`.
  Request-path code returns an error instead; `CatchPanicLayer` is a backstop, not a strategy.
- **Always add context.** Startup failures use `.context(...)` / `.with_context(...)` and say which
  file, key or address failed. Domain error messages carry the value that was rejected
  (`name "admin" is reserved`, not just "invalid"). **review**
- **Map errors to HTTP in one place**: `server/src/error.rs`. `AppError` has one `#[from]` variant
  per module error, and its `IntoResponse` is the only code that chooses a status code. Handlers
  return `Result<_, AppError>` and read request bodies with the `AppJson` extractor, so malformed
  JSON becomes a 400 `AppError` too. Domain errors never mention `StatusCode`. **review**
- **Client messages are fixed strings** (`{"error": "invalid request"}`, `"name is reserved"`,
  `"internal error"`). The full cause chain travels in the `ErrorDetail` response extension, which
  only the request log reads (section 6). **review**

## 4. Module layout

Features live in `server/src/modules/<name>/`. To add one, copy `modules/example/` and then:
declare it in `modules/mod.rs`, add a `#[from]` variant for its error to `AppError`, mount its
`routes()` in `router.rs`, merge its `openapi()` in `swagger.rs`, and add a migration if it needs
tables.

| File | Role |
|---|---|
| `mod.rs` | Wiring: declares the submodules, re-exports the module's public surface, and holds `routes()`, `openapi()` and the `Service` that owns the business rules (e.g. reserved names) on top of the store trait. |
| `handler.rs` | axum handlers with `#[utoipa::path]`: extract, call the `Service`, return `Result<_, AppError>`. No SQL and no business rules. |
| `store.rs` | The store trait and its `sqlx` implementation. The only file in the module that writes SQL. |
| `types.rs` | Request, response and row structs (`serde`, `sqlx::FromRow`, `utoipa::ToSchema` derives). |
| `error.rs` | The module's `thiserror` enum. |

Rules:

- **Keep visibility narrow.** Modules in `lib.rs` are private except what the integration tests
  need (`config`, `store`, and `build_router` / `run` at the root). Handlers are `pub(super)`. An
  item marked `pub` inside a private module is only crate-wide, so use `pub` for items other modules
  need and leave the rest private. **review**
- **A trait needs a reason.** The store trait exists so the `Service` can be unit-tested against a
  mock: it is `#[async_trait]` (the `Service` holds an `Arc<dyn ...>`) plus
  `#[cfg_attr(test, mockall::automock)]`. Mocks are generated; none are checked in. Add a new trait
  only for a real contract between modules or crates; otherwise use the concrete type. **review**
- **`sqlx` queries are runtime queries**: `sqlx::query_as` with `#[derive(sqlx::FromRow)]` structs,
  values passed with `.bind()`. The `query!` family of macros is not used, so `cargo build` needs no
  database and no `.sqlx/` cache. Never build SQL by formatting user input into a string. **review**
- **Migrations** are plain SQL files in `server/migrations/` (`0001_create_examples.sql`, ...). They
  are embedded with `sqlx::migrate!("./migrations")` in `store/sqlite.rs` (`build.rs` reruns the
  build when the folder changes) and applied at startup, so a fresh checkout runs with no extra
  tooling. `make migrate-up` (optional, needs `sqlx-cli`) applies the same files by hand. Never edit
  a migration that has been merged; add a new one. **review**
- **The KV store** (`redb`) is reached through the `KvStore` handle in `store/kv.rs`. Its
  transactions are synchronous, so `KvStore` runs them in `spawn_blocking` (section 5). **review**
- **Shared state is `AppState`** (`state.rs`): the SQLite pool and the `KvStore`, both cheap to
  clone. Handlers take the state they need through `State<...>`: a module's `Service` for module
  routes, `AppState` for `/health` and `/ready`. **review**

## 5. Async

- **Do not block the runtime.** Synchronous file I/O, `redb` transactions and CPU-heavy work go
  through `tokio::task::spawn_blocking`, as `KvStore` does. **review**
- **Never hold a `std::sync::Mutex` or `RwLock` guard across an `.await`.** **tool** clippy
  `await_holding_lock` (in `clippy::all`). If you really need a lock across an await, use
  `tokio::sync::Mutex`, and first ask whether the state needs a lock at all.
- **Handlers must be cancellation-safe.** axum drops the handler future when the client
  disconnects or the timeout layer fires, so any `.await` can be the last thing that runs. Do
  multi-statement writes in one `sqlx` transaction (it rolls back on drop), not as separate
  statements that can stop halfway. **review**
- **Detached tasks** (`tokio::spawn` without joining) log their own errors and stop on shutdown.
  **review**

## 6. Logging

- Use `tracing` events (`info!`, `warn!`, `error!`). No `println!` or `eprintln!` in server code
  (**review**); `dbg!` is rejected by clippy `dbg_macro` (**tool**).
- **Structured fields, constant message**: `info!(%address, "server listening")`, not
  `info!("server listening on {address}")`. **review**
- **Never log request or response bodies, secrets, or raw query strings.** The request log
  (`middleware/request_log.rs`) writes one line per request: method, matched route (not the raw
  path), status, duration, request ID, and the query only when `log_query` is on, limited to
  `query_allowlist` keys with sensitive values redacted. Paths in `skip_paths` (`/health`) are not
  logged. Fields you add must not undo that. **review**
- `#[tracing::instrument]` always uses `skip_all` and names the fields it wants
  (`fields(id = %id)`); the default records every argument with `Debug`, which can leak a body.
  **review**
- The request-log level follows the response status: `error` for 5xx, `warn` for 4xx, `info`
  otherwise. Elsewhere, pick the level by what the operator should do about it. **review**
- **Log an error once.** Handlers return it; the request log records it, with the cause from
  `ErrorDetail`. Do not log and also return the same error. A recovered panic is the one extra
  event (`middleware/recovery.rs`). **review**
- Subscriber setup lives only in `telemetry.rs`: compact output in dev, JSON in prd, this crate at
  `logger.level` and dependencies at `warn`. **review**

## 7. Config and secrets

- **YAML holds operational defaults** in `server/config/config_<env>.yaml`, selected by
  `APP_ENVIRONMENT` (`dev` by default, `prd`; letters, digits, `-` and `_` only, because it becomes
  part of a file name). **Environment variables hold deployment values**: today `SQLITE_DSN`
  (overrides `sqlite.dsn`) and `KV_PATH` (overrides `kv.path`). Deployment-only values are left
  empty in `config_prd.yaml`, so startup fails until the environment supplies them. A new override
  is added in `config.rs`, `.env.example` and `README.md` together. **review**
- **Durations and sizes in YAML carry their unit in the key**: `http.request_timeout_in_second`,
  `http.shutdown_timeout_in_second`, `http.max_body_size_in_bytes`. Only add a timeout key if the
  code enforces it; `axum::serve` exposes no read or idle timeout, so there are none. **review**
- **Unknown keys are rejected.** Every config struct has `#[serde(deny_unknown_fields)]`, so a typo
  fails at startup instead of being ignored. **review**
- **`validate()` runs right after loading** and reports every problem at once, each naming its key
  (`http.port is required`). **review**
- Config is loaded once in `run()` and handed to what needs it (`telemetry::init`, `build_router`,
  the store constructors). No `std::env::var` in handlers, stores or modules; the only reads are
  `APP_ENVIRONMENT` in `lib.rs` and the two overrides in `config.rs`. **review**
- **Secrets never go in YAML or git.** `.env.example` lists names with placeholder values; `.env` is
  git-ignored. **review**

## 8. Testing

- **Unit tests sit beside the code** in a `#[cfg(test)] mod tests` at the bottom of the file.
  Store traits are mocked with `mockall`, so the `Service` is tested without a database.
  **tool** `make test` = `cargo test --workspace`
- **Integration tests live in `server/tests/api/`**: one test binary (`main.rs`), a `helpers.rs`
  with `spawn_app()`, and a file per area (`health.rs`, `example.rs`, `middleware.rs`,
  `swagger.rs`). `spawn_app()` builds the app with `build_router`, binds `127.0.0.1:0`, and gives
  each test its own temp directory holding a SQLite file and a redb file. Tests talk to it over real
  HTTP with `reqwest`. No fixed ports, no state shared between tests. A temp file rather than
  `:memory:`, because each pooled connection to `:memory:` would see a different database.
  **tool** `make test`; the pattern itself is **review**
- Test the contract: status code **and** body for each documented path (201, 400, 409, ...).
  **review**
- Name tests for behaviour: `create_returns_the_persisted_example`. **review**
- Tests may `unwrap()` and `expect()`; see section 3.

## 9. Docs and comments

- **`///` on every public item.** The first sentence is a one-line summary of at most about 15
  words; add `# Errors` and `# Panics` sections when they apply. Modules open with a `//!` summary.
  **tool** `missing_docs` covers everything the crate exports (`config`, `store`, `build_router`,
  `run`) plus the crate-level `//!` of each binary and build script; documenting items in private
  modules is **review**.
- **Every handler has a `#[utoipa::path]` annotation**, and every module exposes `openapi()` that
  `swagger.rs` merges, so `/swagger/openapi.json` matches the real routes. **review**
- **Comments explain why, not what.** Name the constraint, the trade-off or the bug being avoided.
  Explain hard-coded values, or give them a named constant. **review**
- No commented-out code; git has the history. `todo!()` and `unimplemented!()` do not merge.
  **tool** (`todo`, `unimplemented`)

## 10. Dependencies

- **Declare each dependency once**, with its version, in `[workspace.dependencies]` in the root
  `Cargo.toml`. Members write `name.workspace = true` (in `[dependencies]` or `[dev-dependencies]`)
  and add only the `features` they need. **review**
- **`cargo deny check` runs in CI** (`deny.toml`: advisories and yanked crates, a permissive-licence
  allow-list, wildcard versions, registry sources; duplicate versions only warn). Locally it needs
  `cargo-deny` installed. A dependency under a licence outside the allow-list fails the check, which
  forces a deliberate decision. **tool** `make verify`
- **Justify a new crate in the PR**: what it replaces or enables, whether it is maintained, its
  licence, and which default features can be turned off. **review**
- `Cargo.lock` is committed and CI builds with `--locked`. Dependabot opens weekly update PRs for
  cargo, npm and docker; they merge after CI passes.

## Where things live

```
server/
├── build.rs              reruns the build when migrations/ changes (sqlx::migrate! embeds it)
├── config/               config_dev.yaml, config_prd.yaml (selected by APP_ENVIRONMENT)
├── migrations/           sqlx SQL migrations (0001_create_examples.sql, ...)
├── src/
│   ├── main.rs           a few lines: calls lux_server::run()
│   ├── lib.rs            module tree and run(): config → telemetry → stores → router → serve,
│   │                     with graceful shutdown on SIGINT/SIGTERM
│   ├── config.rs         serde structs, load() with SQLITE_DSN / KV_PATH overrides, validate()
│   ├── telemetry.rs      tracing subscriber setup
│   ├── error.rs          AppError (the one place errors become HTTP), AppJson, ErrorBody, ErrorDetail
│   ├── state.rs          AppState: SQLite pool and KvStore
│   ├── router.rs         build_router(): /health, /ready, module routes and the middleware stack
│   ├── swagger.rs        OpenAPI document and the /swagger pages
│   ├── middleware/       request_id.rs, request_log.rs, recovery.rs
│   ├── store/            sqlite.rs (pool, migrations), kv.rs (redb)
│   └── modules/<name>/   mod.rs, handler.rs, store.rs, types.rs, error.rs
└── tests/api/            integration tests: main.rs, helpers.rs, health.rs, example.rs,
                          middleware.rs, swagger.rs
```

## Sources

- [The Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/): formatting; `rustfmt`
  implements it.
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/): naming (casing per
  [RFC 430](https://rust-lang.github.io/rfcs/0430-finalizing-naming-conventions.html), `as_` /
  `to_` / `into_`, getters, constructors) and interoperability.
- [Pragmatic Rust Guidelines](https://microsoft.github.io/rust-guidelines/guidelines/index.html)
  (Microsoft): `M-LINT-OVERRIDE-EXPECT`, `M-PUBLIC-DEBUG`, `M-LOG-STRUCTURED`,
  `M-FIRST-DOC-SENTENCE`, `M-CANONICAL-DOCS`, `M-STATIC-VERIFICATION`.
