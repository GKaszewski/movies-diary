# ----- spa -----
FROM node:22-slim AS spa-builder
WORKDIR /spa
COPY spa/package.json spa/package-lock.json ./
RUN npm ci
COPY spa/ .
RUN npm run build

# ----- chef base -----
FROM rust:slim-bookworm AS chef
# libwebp-dev: required at build time by the `webp` crate (C bindings)
RUN apt-get update && apt-get install -y --no-install-recommends \
    libwebp-dev \
    pkg-config \
 && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --locked --version 0.1.78
WORKDIR /build

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ----- build -----
FROM chef AS builder

# Features: federation (ActivityPub), nats (EVENT_BUS_BACKEND=nats).
# Build without federation:  --build-arg FEATURES=
# To add NATS support:       --build-arg FEATURES=federation,nats
ARG FEATURES=federation

COPY .cargo ./.cargo
COPY --from=planner /build/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    cargo chef cook --release -p server -p worker --no-default-features --features "${FEATURES}" --recipe-path recipe.json

# All sqlx queries use the runtime API (no query! macros), so no database
# or .sqlx cache is needed at compile time.
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    cargo build --release -p server -p worker --no-default-features --features "${FEATURES}"

# ----- runtime -----
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    wget \
    libwebp7 \
    fonts-dejavu-core \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /build/target/release/server ./server
COPY --from=builder /build/target/release/worker ./worker
COPY static ./static
COPY --from=spa-builder /spa/dist ./spa/dist

EXPOSE 3000

ENV RUST_LOG=server=info,worker=info,bootstrap=info,config=info,http_axum=info,tower_http=info

CMD ["./server"]
