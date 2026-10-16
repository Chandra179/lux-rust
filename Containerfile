FROM docker.io/library/rust:1.99.1-bookworm AS build

WORKDIR /src

# Dependency layer: compile third-party crates against stub sources so this
# layer is reused until Cargo.toml or Cargo.lock change.
COPY Cargo.toml Cargo.lock ./
COPY server/Cargo.toml server/Cargo.toml
COPY crates/example/Cargo.toml crates/example/Cargo.toml
RUN mkdir server/src crates/example/src \
    && echo 'fn main() {}' > server/src/main.rs \
    && touch crates/example/src/lib.rs \
    && cargo build --release --locked

# Cargo decides freshness by mtime and COPY keeps the context's (older)
# timestamps, so touch the sources to force a rebuild over the stubs.
COPY . .
RUN find server/src crates/example/src -type f -name '*.rs' -exec touch {} + \
    && cargo build --release --locked \
    && strip target/release/lux-server

FROM docker.io/library/debian:bookworm-slim

RUN groupadd --system lux \
    && useradd --system --gid lux --no-create-home --shell /usr/sbin/nologin lux \
    && mkdir -p /data \
    && chown lux:lux /data

WORKDIR /app
COPY --from=build --chown=lux:lux /src/target/release/lux-server /app/lux-server
COPY --from=build --chown=lux:lux /src/server/config /app/server/config

ENV APP_ENVIRONMENT=prd \
    SQLITE_DSN=/data/lux.db

EXPOSE 8080
VOLUME ["/data"]
USER lux
ENTRYPOINT ["/app/lux-server"]
