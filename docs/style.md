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
| Today's set: `unsafe_code = forbid`, `missing_debug_implementations`, `missing_docs`, clippy `all`, `mod_module_files`, `unwrap_used`, `expect_used`, `dbg_macro`, `todo` and `unimplemented` (all `warn`, so `-D warnings` makes them errors). `Cargo.toml` is the source of truth. | **tool** clippy / rustc |
| Edition 2024 and Rust 1.99.1 are pinned: the edition and `rust-version` in `[workspace.package]`, the toolchain (with `rustfmt` and `clippy`) in `rust-toolchain.toml`. Raising either is its own PR. | **tool** rustup selects the pinned toolchain in this repo |
| Override a lint locally with `#[expect(lint, reason = "...")]`, not `#[allow]`; a stale `expect` then warns on its own. The test-code exemption in section 3 is the only crate-level `allow`. | **review** |
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
  secret or a handle that has no `Debug` of its own implements it by hand with
  `finish_non_exhaustive()` so nothing sensitive is printed. **review**
- Take `&str` / `&[T]` in parameters, not `&String` / `&Vec<T>`. **tool** clippy `ptr_arg`
- JSON object keys are `snake_case` (serde's default; `Example` serialises as `id`, `name`,
  `created_at`). Do not switch them to `PascalCase` or `camelCase` with `#[serde(rename_all)]`; the
  React client mirrors the Rust names. **review**

## 3. Errors

- **One central error type**: `AppError` in `error.rs`, a `thiserror` enum whose variants are
  HTTP-level categories (`InvalidRequest`, `Conflict`, `Database`, ...), as in realworld-axum-sqlx.
  Underlying errors are wrapped with `#[from]` so they stay the `source()`. A new kind of failure is
  a new variant. A resource gets its own error enum only when callers must tell several of its
  failures apart, and that enum converts into `AppError` with `From`. **review**
- **`anyhow` is only for the binary edge**: `main`, `run()` and startup code. Modules never return
  `anyhow::Error`. **review**
- **No `unwrap()` or `expect()` outside tests and startup.** `unwrap_used` and `expect_used` are
  `warn` in `[workspace.lints]`, so `-D warnings` fails the build. **tool** Startup code does not
  need them: it returns errors with `?`. Test code is exempt through one attribute at the top of each
  crate's `lib.rs`: `#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]`.
  Request-path code returns an error instead of panicking.
- **Always add context.** Startup failures use `.context(...)` / `.with_context(...)` and say which
  file, key or address failed. An error's `Display` text says what failed and with which value; the
  text sent to the client is separate and fixed. **review**
- **Map errors to HTTP in one place**: `server/src/error.rs`. `AppError`'s `IntoResponse` is the
  only code that chooses a status code for a failure. Handlers return `Result<_, AppError>` and read
  request bodies with the `AppJson` extractor, so malformed JSON becomes a 400 `AppError` too.
  Handlers never build error responses by hand. **review**
- **Client messages are fixed strings** (`{"error": "invalid request"}`, `"name is reserved"`,
  `"internal error"`), so no internal detail or user input leaks into a response. **review**

## 4. Code layout

Rust has no mandated project layout. This workspace has two kinds of crate, as in the
rust10x/rust-web-app template (a web server crate plus library crates): `server/` is the web
server, and `crates/<name>/` holds a feature's data type, rules and SQL with no HTTP in it. Inside
the server, top-level files are named for their role (`config.rs`, `startup.rs`, `db.rs`) and there
is one file per resource under `routes/`, as in zero-to-production and realworld-axum-sqlx. There
are no `handlers/`, `services/` or `models/` layers.

**Crates and routes**

- **A feature crate** (`crates/example`) has its own `Cargo.toml`, is a member of the workspace and
  is listed in `[workspace.dependencies]` as a path dependency. It exports the type, a `create`-style
  function taking `&SqlitePool`, and its own `thiserror` error enum. It never depends on `server`.
  To add one: create `crates/<name>/`, add it to `members` and `[workspace.dependencies]` in the root
  `Cargo.toml`, depend on it from `server`, and add its `Cargo.toml` and a stub `src/lib.rs` to the
  dependency layer of the `Containerfile`.
- **One file per resource in `server/src/routes/`** (`example.rs`, `health.rs`) holds that
  resource's `router()`, its request type and its handlers. To add a resource: create
  `routes/<name>.rs`, declare it in `routes.rs` and merge its `router()` there, convert the crate's
  error in `error.rs` (`impl From<ExampleError> for AppError`), and add a migration if it needs
  tables.
- **Grow by splitting the file, not by adding layers.** When a route file gets long it becomes
  `routes/<name>.rs` plus a `routes/<name>/` folder of submodules.
- **Handlers are thin**: extract, call the crate, return `Result<_, AppError>`. Rules live in the
  feature crate as plain functions (`validate_name`) so a unit test can call them without HTTP.
  **review**
- **Concrete types, no traits by default.** Handlers take `State<SqlitePool>` and call the crate's
  functions directly. Add a trait only when there is a real second implementation or an external
  system you must fake (a mail provider), never to mock our own database access: SQL is tested
  against a real SQLite database (section 8). axum's dependency-injection example shows the
  trait-object pattern for the cases that do need it. **review**
- **Shared state is the `SqlitePool`**, which is cheap to clone. Resource routers are
  `Router<SqlitePool>`, merged in `routes.rs`, with the state attached once in `build_router`
  (`startup.rs`). When a second shared resource appears, group them in an `AppState` struct that
  derives `FromRef`, so a handler can still ask for just the piece it needs. **review**

**Files and visibility**

- **No `mod.rs`.** A module `foo` is `foo.rs`, and its children are `foo/bar.rs`. The Rust Book calls
  this the idiomatic style, and it keeps editor tabs distinguishable. **tool** clippy
  `mod_module_files`.
- **Keep visibility narrow.** Modules in `server/src/lib.rs` are private except `config` and `db`,
  and `run` at the root. An item marked `pub` inside a private module is only crate-wide, so use
  `pub` where another module needs it and leave the rest private. **review**

**Middleware**

- **Middleware is tower.** The server currently uses two layers, wired in `build_router`
  (`startup.rs`): axum's `DefaultBodyLimit` and tower-http's `TimeoutLayer`. When app-specific
  middleware is needed, write it as an `async fn(Request, Next) -> Response` for
  `axum::middleware::from_fn` (or `from_fn_with_state`), one file per concern in a `middleware/`
  folder (declared by `middleware.rs`, no `mod.rs`). Write a full `tower::Service` only to publish a
  layer. **review**
- **Assemble the stack once**, in `build_router`. With more than two layers use `ServiceBuilder`: the
  first layer listed is the outermost, so the order reads top to bottom. **review**

**Data**

- **`sqlx` queries are runtime queries**: `sqlx::query_as` with `#[derive(sqlx::FromRow)]` structs,
  values passed with `.bind()`. The `query!` family of macros is not used, so `cargo build` needs no
  database and no `.sqlx/` cache. Never build SQL by formatting user input into a string. **review**
- **Migrations** are plain SQL files in `server/migrations/` (`0001_create_examples.sql`, ...). They
  are embedded with `sqlx::migrate!("./migrations")` in `db.rs` (`build.rs` reruns the build when the
  folder changes) and applied at startup, so a fresh checkout runs with no extra tooling.
  `make migrate-up` (optional, needs `sqlx-cli`) applies the same files by hand. Never edit a
  migration that has been merged; add a new one. **review**

## 5. Async

- **Do not block the runtime.** Synchronous file I/O, blocking libraries and CPU-heavy work go
  through `tokio::task::spawn_blocking`. **review**
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

There is no logging or telemetry setup yet, on purpose. The server prints one startup line with
`println!`, and the default panic hook writes panic messages to stderr. `dbg!` is rejected by clippy
`dbg_macro` (**tool**). When logging is added:

- Use `tracing` with one subscriber setup in a single `telemetry.rs`. **review**
- Use structured fields and a constant message (`info!(%address, "server listening")`). **review**
- Never log request or response bodies, secrets, or raw query strings. **review**
- `#[tracing::instrument]` always uses `skip_all` and names the fields it wants. **review**
- Log an error once: either return it or log it, not both. **review**

## 7. Config and secrets

- **YAML holds operational defaults** in `server/config/config_<env>.yaml`, selected by
  `APP_ENVIRONMENT` (`dev` by default, `prd`; letters, digits, `-` and `_` only, because it becomes
  part of a file name). **Environment variables hold deployment values**: today `SQLITE_DSN`
  (overrides `sqlite.dsn`). Deployment-only values are left empty in `config_prd.yaml`, so startup
  fails until the environment supplies them. A new override
  is added in `config.rs`, `.env.example` and `README.md` together. **review**
- **Durations and sizes in YAML carry their unit in the key**: `http.request_timeout_in_second`,
  `http.shutdown_timeout_in_second`, `http.max_body_size_in_bytes`. Only add a timeout key if the
  code enforces it; `axum::serve` exposes no read or idle timeout, so there are none. **review**
- **Unknown keys are rejected.** Every config struct has `#[serde(deny_unknown_fields)]`, so a typo
  fails at startup instead of being ignored. **review**
- **`validate()` runs right after loading** and reports every problem at once, each naming its key
  (`http.port is required`). **review**
- Config is loaded once in `run()` and handed to what needs it (`build_router`, `db::connect`). No
  `std::env::var` in handlers or stores; the only reads are `APP_ENVIRONMENT` in `startup.rs` and
  the `SQLITE_DSN` override in `config.rs`. **review**
- **Secrets never go in YAML or git.** `.env.example` lists names with placeholder values; `.env` is
  git-ignored. **review**

## 8. Testing

- **Unit tests sit beside the code** in a `#[cfg(test)] mod tests` at the bottom of the file. Pure
  rules (`validate_name`) are tested directly. **tool** `make test` = `cargo test --workspace`
- **SQL is tested against a real database with `#[sqlx::test]`.** The macro gives each test its own
  fresh SQLite file with the migrations applied (`#[sqlx::test(migrations =
  "../../server/migrations")]` in a crate under `crates/`), so a test takes a `SqlitePool` argument
  and calls the function under test (`create`). No mocks of our own queries. **tool** `make test`
- There is no `tests/` directory and no HTTP-level test yet. When one is needed, use a multi-file
  integration test at `server/tests/api/main.rs` that builds the app with `build_router`, binds
  `127.0.0.1:0`, and gives each test its own temp SQLite file (a temp file rather than `:memory:`,
  because each pooled connection to `:memory:` sees a different database). **review**
- Name tests for behaviour: `create_persists_a_row`. **review**
- Tests may `unwrap()` and `expect()`; see section 3.

## 9. Docs and comments

- **`///` on every public item.** The first sentence is a one-line summary of at most about 15
  words; add `# Errors` and `# Panics` sections when they apply. Modules open with a `//!` summary.
  **tool** `missing_docs` covers everything a crate exports (in `server`: `config`, `db`, `run`; in a feature crate: every `pub` item) plus the crate-level `//!` of each
  binary and build script; documenting items in private modules is **review**.
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
crates/
└── example/              feature crate: Example type, rules, SQL, its own error enum
server/
├── build.rs              reruns the build when migrations/ changes (sqlx::migrate! embeds it)
├── config/               config_dev.yaml, config_prd.yaml (selected by APP_ENVIRONMENT)
├── migrations/           sqlx SQL migrations (0001_create_examples.sql, ...)
└── src/
    ├── main.rs           a few lines: calls lux_server::run()
    ├── lib.rs            module tree and the public API (run)
    ├── startup.rs        build_router() with body limit and timeout, run(): config → database →
    │                     router → serve, graceful shutdown on SIGINT/SIGTERM
    ├── config.rs         serde structs, load() with the SQLITE_DSN override, validate()
    ├── error.rs          AppError (the one place errors become HTTP), AppJson
    ├── db.rs             SQLite pool and embedded migrations
    ├── routes.rs         declares the route modules, router() merging them
    └── routes/           one file per resource: health.rs, example.rs
```

## Sources

Layout and structure:

- [zero-to-production](https://github.com/LukeMathWalker/zero-to-production/tree/main/src) and
  [realworld-axum-sqlx](https://github.com/launchbadge/realworld-axum-sqlx/tree/main/src/http):
  `main.rs` + `lib.rs`, role-named top-level files, one file per route or resource.
- [rust10x/rust-web-app](https://github.com/rust10x/rust-web-app): a workspace with a web-server
  crate under `services/` and library crates under `libs/`.
- [The Rust Book, ch. 7.5](https://doc.rust-lang.org/book/ch07-05-separating-modules-into-different-files.html):
  `foo.rs` + `foo/bar.rs` instead of `mod.rs`.
- [Cargo package layout](https://doc.rust-lang.org/cargo/guide/project-layout.html): `src/main.rs`,
  `src/lib.rs`, `tests/`.
- [axum middleware](https://docs.rs/axum/latest/axum/middleware/index.html): `from_fn`, tower layers,
  `ServiceBuilder` ordering.
- [axum dependency-injection example](https://github.com/tokio-rs/axum/blob/main/examples/dependency-injection/src/main.rs):
  when a trait object is the right tool.
- [`#[sqlx::test]`](https://docs.rs/sqlx/latest/sqlx/attr.test.html): a fresh database per test.

Style and naming:

- [The Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/): formatting; `rustfmt`
  implements it.
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/): naming (casing per
  [RFC 430](https://rust-lang.github.io/rfcs/0430-finalizing-naming-conventions.html), `as_` /
  `to_` / `into_`, getters, constructors) and interoperability.
- [Pragmatic Rust Guidelines](https://microsoft.github.io/rust-guidelines/guidelines/index.html)
  (Microsoft): `M-LINT-OVERRIDE-EXPECT`, `M-PUBLIC-DEBUG`, `M-LOG-STRUCTURED`,
  `M-FIRST-DOC-SENTENCE`, `M-CANONICAL-DOCS`, `M-STATIC-VERIFICATION`.
