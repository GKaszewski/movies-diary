# ----- build -----
FROM rust:slim-bookworm AS builder

WORKDIR /build

# Cache dependency compilation separately from source
COPY Cargo.toml Cargo.lock ./
COPY .cargo ./.cargo
COPY .sqlx ./.sqlx
COPY crates/adapters/activitypub/Cargo.toml       crates/adapters/activitypub/Cargo.toml
COPY crates/adapters/activitypub-base/Cargo.toml  crates/adapters/activitypub-base/Cargo.toml
COPY crates/adapters/auth/Cargo.toml              crates/adapters/auth/Cargo.toml
COPY crates/adapters/event-payload/Cargo.toml     crates/adapters/event-payload/Cargo.toml
COPY crates/adapters/event-publisher/Cargo.toml   crates/adapters/event-publisher/Cargo.toml
COPY crates/adapters/nats/Cargo.toml              crates/adapters/nats/Cargo.toml
COPY crates/adapters/metadata/Cargo.toml          crates/adapters/metadata/Cargo.toml
COPY crates/adapters/poster-fetcher/Cargo.toml    crates/adapters/poster-fetcher/Cargo.toml
COPY crates/adapters/poster-storage/Cargo.toml    crates/adapters/poster-storage/Cargo.toml
COPY crates/adapters/poster-sync/Cargo.toml       crates/adapters/poster-sync/Cargo.toml
COPY crates/adapters/export/Cargo.toml            crates/adapters/export/Cargo.toml
COPY crates/adapters/importer/Cargo.toml          crates/adapters/importer/Cargo.toml
COPY crates/adapters/rss/Cargo.toml               crates/adapters/rss/Cargo.toml
COPY crates/adapters/sqlite/Cargo.toml              crates/adapters/sqlite/Cargo.toml
COPY crates/adapters/sqlite-federation/Cargo.toml   crates/adapters/sqlite-federation/Cargo.toml
COPY crates/adapters/sqlite-event-queue/Cargo.toml  crates/adapters/sqlite-event-queue/Cargo.toml
COPY crates/adapters/postgres/Cargo.toml            crates/adapters/postgres/Cargo.toml
COPY crates/adapters/postgres-federation/Cargo.toml crates/adapters/postgres-federation/Cargo.toml
COPY crates/adapters/postgres-event-queue/Cargo.toml crates/adapters/postgres-event-queue/Cargo.toml
COPY crates/adapters/template-askama/Cargo.toml   crates/adapters/template-askama/Cargo.toml
COPY crates/application/Cargo.toml                crates/application/Cargo.toml
COPY crates/domain/Cargo.toml                     crates/domain/Cargo.toml
COPY crates/presentation/Cargo.toml               crates/presentation/Cargo.toml
COPY crates/doc/Cargo.toml                        crates/doc/Cargo.toml
COPY crates/tui/Cargo.toml                        crates/tui/Cargo.toml
COPY crates/worker/Cargo.toml                     crates/worker/Cargo.toml

# Stub every crate so cargo can resolve and fetch deps
RUN find crates -name "Cargo.toml" | sed 's|/Cargo.toml||' | \
    xargs -I{} sh -c 'mkdir -p {}/src && echo "fn main(){}" > {}/src/main.rs && echo "" > {}/src/lib.rs'

RUN cargo fetch

# Now copy real sources (invalidates cache only on source changes)
COPY crates ./crates

# .cargo/config.toml sets SQLX_OFFLINE=true; .sqlx contains the pre-verified query cache.
# No live database needed at compile time.
#
# To build with PostgreSQL backend instead:
#   --build-arg FEATURES=postgres,postgres-federation
# To add NATS support (EVENT_BUS_BACKEND=nats):
#   --build-arg FEATURES=sqlite,sqlite-federation,nats
ARG FEATURES=sqlite,sqlite-federation
RUN cargo build --release -p presentation -p worker --no-default-features --features "${FEATURES}"

# ----- runtime -----
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /build/target/release/presentation ./presentation
COPY --from=builder /build/target/release/worker ./worker
COPY static ./static

EXPOSE 3000

ENV RUST_LOG=presentation=info,tower_http=info

CMD ["./presentation"]
