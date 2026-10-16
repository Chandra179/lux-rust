## Lux

Lux contains a Rust API server and a React frontend.

### Prerequisites

- Rust, installed with [rustup](https://rustup.rs). `rust-toolchain.toml` pins the
  toolchain and rustup picks it up automatically.
- Node 22, for the frontend.
- [podman](https://podman.io), to build the container image (`make podman-build`).
- [cargo-deny](https://crates.io/crates/cargo-deny), for `make verify` (the
  dependency audit that CI also runs): `cargo install cargo-deny --locked`.
- [sqlx-cli](https://crates.io/crates/sqlx-cli), optional: only for
  `make migrate-up` (see below).

### Backend

```bash
make run
```

Run it from the repository root. The API listens on `http://localhost:8080`.
Health, readiness, and Swagger are available at `/health`, `/ready`, and
`/swagger/index.html`. The Swagger page loads its UI from a CDN, so it needs
internet access.

Settings come from `server/config/config_<env>.yaml`, where `<env>` is the value
of `APP_ENVIRONMENT` (`dev` by default). Two values can be overridden with
environment variables: `SQLITE_DSN` (SQLite database, `sqlite.dsn` in the YAML)
and `KV_PATH` (embedded key-value store, `kv.path`). `make run` defaults them to
`lux.db` and `lux.redb`. `config_prd.yaml` leaves both empty on purpose, so set
them when running with `APP_ENVIRONMENT=prd`.

The SQL migrations in `server/migrations` are embedded in the server and applied
at startup, so a fresh checkout needs no extra step. To apply the same files to
the database in `SQLITE_DSN` by hand, install sqlx-cli and run:

```bash
make migrate-up
```

### Frontend

Install dependencies and start Vite:

```bash
make web-install
make web-dev
```

The frontend listens on `http://localhost:5173` and proxies `/api/*` to the Rust
server. The API client can be pointed at another server with
`web/.env.local`:

```bash
VITE_API_BASE_URL=/api
```

Frontend checks:

```bash
make web-lint
make web-typecheck
make web-build
```

### Layout

```
.
├── server/        Rust API server (crate lux-server)
│   ├── config/        config_dev.yaml, config_prd.yaml
│   ├── migrations/    SQL migrations
│   ├── src/           lib, router, config, modules/<name>/, store/, middleware/
│   └── tests/api/     integration tests
├── web/           React frontend (Vite)
├── docs/
│   └── style.md   Rust style guide
├── Cargo.toml     workspace root: shared dependencies and lints
└── Makefile
```

### Code style

Rust code follows [docs/style.md](docs/style.md), which also maps out where things
live in `server/src`.
