# Movies Diary

A self-hosted, server-side rendered movie logging system with a full REST API. Built in Rust — no JavaScript in the HTML interface, just HTML forms and an RSS feed. Designed to run as a lightweight widget embedded on a personal site or as a backend for third-party clients.

## Features

- Log movies with a TMDB/OMDb ID or manual title/year/director, with a 0–5 rating
- Immutable append-only viewing ledger (tracks re-watches)
- Background poster fetching and storage (local filesystem or S3-compatible)
- RSS/Atom feed for public subscription (global and per-user)
- JWT authentication via cookie (HTML) or Bearer token (REST API)
- ActivityPub federation — follow/unfollow remote users on any compatible server, accept/reject/remove followers, pending follow request management
- CSV and JSON diary export
- File importer: upload CSV, TSV, JSON, or XLSX from any source (Letterboxd, IMDb, etc.), map columns to domain fields via a step-by-step wizard or REST API, save mapping profiles for repeat imports
- REST API v1 (`/api/v1/`) with full feature parity with the HTML interface
- OpenAPI documentation at `/docs` (Swagger UI) and `/scalar` (Scalar)
- CSRF protection on all HTML form routes (double-submit cookie, defense-in-depth on top of `SameSite=Strict`)
- Per-IP rate limiting via token bucket (production-grade, backed by `axum-governor`)
- Terminal UI client (`crates/tui`) for logging reviews, bulk CSV import, and diary browsing

## Architecture

Hexagonal (Ports & Adapters) with Domain-Driven Design:

```
domain              — pure types and trait definitions, no external deps
application         — use cases / business logic orchestration
presentation        — Axum HTTP router, composition root for the HTTP process
worker              — standalone worker binary (event consumer, poster sync, federation)
adapters/
  auth                 — JWT issuance and validation (Argon2 passwords)
  sqlite               — SQLite repository + connection factory
  postgres             — PostgreSQL repository + connection factory
  metadata             — TMDB / OMDb HTTP client
  poster-fetcher       — downloads poster images
  poster-storage       — stores posters on local filesystem or S3-compatible storage
  poster-sync          — event handler: triggers poster fetch+store on MovieDiscovered
  template-askama      — Askama HTML rendering
  rss                  — RSS/Atom feed generation
  export               — CSV and JSON diary serialization
  importer             — CSV/TSV/JSON/XLSX parser and column mapper for bulk import
  event-payload        — shared event serialization DTOs (used by all event bus adapters)
  sqlite-event-queue   — durable polling event queue backed by SQLite
  postgres-event-queue — durable polling event queue backed by PostgreSQL
  nats                 — NATS Core / JetStream event publisher and consumer
  event-publisher      — in-memory event channel (tests only)
  activitypub          — ActivityPub federation wiring (follow, inbox/outbox, actor)
  activitypub-base     — core ActivityPub protocol types and service
  sqlite-federation    — SQLite-backed federation repository
  postgres-federation  — PostgreSQL-backed federation repository
doc                 — OpenAPI spec assembly and Swagger UI / Scalar serving
tui                 — terminal UI client (ratatui)
```

## Prerequisites

- Rust (stable, 2024 edition)
- SQLite
- Poster storage: local filesystem (zero deps) or an S3-compatible object store (e.g. MinIO)
- An [OMDb API key](https://www.omdbapi.com/apikey.aspx)

## Environment Variables

A `.env.example` file is provided at the repo root — copy it to `.env` and fill in your values.

```env
# Database
DATABASE_URL=sqlite://movies.db

# Authentication
JWT_SECRET=change-me

# OMDb metadata
OMDB_API_KEY=your-key

# Public base URL (used for ActivityPub actor URLs and canonical links)
BASE_URL=https://yourdomain.example.com

# Image storage — pick one backend:

# Option A: local filesystem (zero deps)
IMAGE_STORAGE_BACKEND=local
IMAGE_STORAGE_PATH=./images

# Option B: S3-compatible (MinIO, AWS S3, etc.)
# IMAGE_STORAGE_BACKEND=s3
# MINIO_ENDPOINT=http://localhost:9000
# MINIO_BUCKET=posters
# MINIO_REGION=minio
# MINIO_ACCESS_KEY_ID=minioadmin
# MINIO_SECRET_ACCESS_KEY=minioadmin

# Optional
HOST=0.0.0.0
PORT=3000
RATE_LIMIT=60           # requests per minute per IP (default: 60)
ALLOW_REGISTRATION=true # set to false to disable new sign-ups
SECURE_COOKIES=true     # set when serving over HTTPS
RUST_LOG=presentation=info,tower_http=info,worker=info,application=info

# Event bus — "db" (default, uses same database) or "nats"
EVENT_BUS_BACKEND=db
# NATS_URL=nats://localhost:4222   # required when EVENT_BUS_BACKEND=nats
```

The `worker` binary must run alongside `presentation` to process events:

```bash
cargo run -p worker
```

## Run

```bash
cargo run -p presentation   # HTTP server (0.0.0.0:3000)
cargo run -p worker         # event worker (poster sync, in a separate terminal)
```

The worker polls the event queue and must run alongside the presentation to process background tasks like poster fetching. Both processes share the same database.

## API

All REST endpoints are under `/api/v1/`. Authentication uses `Authorization: Bearer <token>` obtained from `POST /api/v1/auth/login`.

Interactive API documentation is available at runtime:

- **Swagger UI** — `http://localhost:3000/docs`
- **Scalar** — `http://localhost:3000/scalar`

## Terminal UI

```bash
cargo run -p tui
```

Supports review logging, bulk CSV import (column order matches the export format), and diary browsing with review history.

## Test

```bash
cargo test
```

## Docker

The image contains both `presentation` (HTTP server) and `worker` (event processor). Run them as separate containers sharing the same data volume:

```bash
# Build (SQLite + federation + NATS support)
docker build -t movies-diary \
  --build-arg FEATURES=sqlite,sqlite-federation,nats .

# HTTP server
docker run -p 3000:3000 \
  -e DATABASE_URL=sqlite:///data/movies.db \
  -e JWT_SECRET=change-me \
  -e OMDB_API_KEY=your-key \
  -e BASE_URL=https://yourdomain.example.com \
  -e EVENT_BUS_BACKEND=nats \
  -e NATS_URL=nats://nats:4222 \
  -v $(pwd)/data:/data \
  movies-diary

# Event worker (separate container, same image)
docker run \
  -e DATABASE_URL=sqlite:///data/movies.db \
  -e JWT_SECRET=change-me \
  -e OMDB_API_KEY=your-key \
  -e BASE_URL=https://yourdomain.example.com \
  -e EVENT_BUS_BACKEND=nats \
  -e NATS_URL=nats://nats:4222 \
  -v $(pwd)/data:/data \
  --entrypoint ./worker \
  movies-diary
```

To build for PostgreSQL: `--build-arg FEATURES=postgres,postgres-federation,nats`

## License

MIT License. See [LICENSE](LICENSE).
