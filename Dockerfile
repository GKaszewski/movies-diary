# ----- build -----
FROM rust:slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends sqlite3 && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Cache dependency compilation separately from source
COPY Cargo.toml Cargo.lock ./
COPY crates/adapters/activitypub/Cargo.toml       crates/adapters/activitypub/Cargo.toml
COPY crates/adapters/auth/Cargo.toml              crates/adapters/auth/Cargo.toml
COPY crates/adapters/event-publisher/Cargo.toml   crates/adapters/event-publisher/Cargo.toml
COPY crates/adapters/metadata/Cargo.toml          crates/adapters/metadata/Cargo.toml
COPY crates/adapters/poster-fetcher/Cargo.toml    crates/adapters/poster-fetcher/Cargo.toml
COPY crates/adapters/poster-storage/Cargo.toml    crates/adapters/poster-storage/Cargo.toml
COPY crates/adapters/rss/Cargo.toml               crates/adapters/rss/Cargo.toml
COPY crates/adapters/sqlite/Cargo.toml            crates/adapters/sqlite/Cargo.toml
COPY crates/adapters/template-askama/Cargo.toml   crates/adapters/template-askama/Cargo.toml
COPY crates/application/Cargo.toml                crates/application/Cargo.toml
COPY crates/domain/Cargo.toml                     crates/domain/Cargo.toml
COPY crates/presentation/Cargo.toml               crates/presentation/Cargo.toml
COPY crates/tui/Cargo.toml                        crates/tui/Cargo.toml

# Stub every crate so cargo can resolve and fetch deps
RUN find crates -name "Cargo.toml" | sed 's|/Cargo.toml||' | \
    xargs -I{} sh -c 'mkdir -p {}/src && echo "fn main(){}" > {}/src/main.rs && echo "" > {}/src/lib.rs'

RUN cargo fetch

# Now copy real sources (invalidates cache only on source changes)
COPY crates ./crates

# sqlx macros verify queries at compile time; create a real DB from migrations
RUN sqlite3 /build/dev.db \
      < crates/adapters/sqlite/migrations/0001_initial.sql && \
    sqlite3 /build/dev.db \
      < crates/adapters/sqlite/migrations/0002_users.sql && \
    sqlite3 /build/dev.db \
      < crates/adapters/sqlite/migrations/0003_activitypub.sql && \
    sqlite3 /build/dev.db \
      < crates/adapters/sqlite/migrations/0004_username.sql && \
    sqlite3 /build/dev.db \
      < crates/adapters/sqlite/migrations/0005_activitypub_v2.sql

ENV DATABASE_URL=sqlite:///build/dev.db

RUN cargo build --release -p presentation

# ----- runtime -----
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /build/target/release/presentation ./presentation
COPY static ./static

EXPOSE 3000

ENV RUST_LOG=presentation=info,tower_http=info

CMD ["./presentation"]
