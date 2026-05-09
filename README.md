# Movies Diary

A self-hosted, server-side rendered movie logging system. Built in Rust — no JavaScript, no SPA, just HTML forms and an RSS feed. Designed to run as a lightweight widget embedded on a personal site.

## Features

- Log movies with a TMDB/OMDb ID and a 0–5 rating
- Immutable append-only viewing ledger (tracks re-watches)
- Background poster fetching and storage (local filesystem or S3-compatible)
- RSS/Atom feed for public subscription
- JWT authentication via cookie (HTML) or Bearer token (REST API)
- Zero JavaScript

## Architecture

Hexagonal (Ports & Adapters) with Domain-Driven Design:

```
domain        — pure types and trait definitions, no external deps
application   — use cases / business logic orchestration
presentation  — Axum HTTP router, wires all adapters together
adapters/
  auth        — JWT issuance and validation (Argon2 passwords)
  sqlite      — SQLite repository via sqlx
  metadata    — OMDb HTTP client
  poster-fetcher — downloads poster images
  poster-storage — uploads posters to local filesystem or S3-compatible storage
  template-askama — Askama HTML rendering
  rss         — RSS/Atom feed generation
  event-publisher — async event channel for background poster sync
```

## Prerequisites

- Rust (stable, 2024 edition)
- SQLite
- Poster storage: local filesystem (zero deps) or an S3-compatible object store (e.g. MinIO)
- An [OMDb API key](https://www.omdbapi.com/apikey.aspx)

## Environment Variables

A `.env.example` file is provided at the repo root — copy it to `.env` and fill in your values. Key variables:

```env
# Database
DATABASE_URL=sqlite://movies.db

# Authentication
JWT_SECRET=change-me

# OMDb metadata
OMDB_API_KEY=your-key

# Poster storage — pick one backend:

# Option A: local filesystem (zero deps)
POSTER_STORAGE_BACKEND=local
POSTER_STORAGE_PATH=./posters

# Option B: S3-compatible (MinIO, AWS S3, etc.)
# POSTER_STORAGE_BACKEND=s3
# MINIO_ENDPOINT=http://localhost:9000
# MINIO_BUCKET=posters
# MINIO_REGION=minio
# MINIO_ACCESS_KEY_ID=minioadmin
# MINIO_SECRET_ACCESS_KEY=minioadmin
```

See `.env.example` for optional variables (rate limiting, logging, host/port, etc.).

## Run

```bash
cargo run -p presentation
```

Server listens on `0.0.0.0:3000`.

## Test

```bash
cargo test
```

## License

MIT License. See [LICENSE](LICENSE).
