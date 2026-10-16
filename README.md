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
Health and readiness are at `/health` and `/ready`, and `POST /example` creates an
example record.

Settings come from `server/config/config_<env>.yaml`, where `<env>` is the value
of `APP_ENVIRONMENT` (`dev` by default). The database location can be overridden
with the `SQLITE_DSN` environment variable (`sqlite.dsn` in the YAML); `make run`
defaults it to `lux.db`. `config_prd.yaml` leaves it empty on purpose, so set it
when running with `APP_ENVIRONMENT=prd`.

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
│   └── src/           startup, config, db, error, routes/ (one file per resource)
├── crates/
│   └── example/   example feature as its own crate (type, rules, SQL)
├── web/           React frontend (Vite)
├── docs/
│   └── style.md   Rust style guide
├── scripts/
│   └── rename-project.sh   renames the project (see "Using this template")
├── Cargo.toml     workspace root: shared dependencies and lints
└── Makefile
```

### Code style

Rust code follows [docs/style.md](docs/style.md), which also maps out where things
live in `server/src`.

### Using this template

To rename the project from Lux to your own name, run this from the repository root:

```bash
scripts/rename-project.sh acme
```

The name is lowercase letters, digits, `-` and `_`. The script renames the crate and
binary (`acme-server`), the library (`acme_server`), the web package, the default
`acme.db` file name, and the visible "Lux" titles. It then refreshes
`Cargo.lock` and runs `cargo fmt`. It needs GNU sed (on macOS, install `gnu-sed`) and
is safe to run twice. Review `git diff` afterwards and run `make test`.

This template does not include a `LICENSE`; add the one you want.
