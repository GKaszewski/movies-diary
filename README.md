# Movies Diary

A self-hosted, server-side rendered movie logging system with a full REST API. Built in Rust — no JavaScript in the HTML interface, just HTML forms and an RSS feed. Designed to run as a lightweight widget embedded on a personal site or as a backend for third-party clients.

## Features

- Log movies with a TMDB/OMDb ID or manual title/year/director, with a 0–5 rating
- Immutable append-only viewing ledger (tracks re-watches)
- Background poster fetching and storage (local filesystem or S3-compatible)
- Movie enrichment via TMDb — full cast, crew, genres, keywords, runtime, budget/revenue, ratings; fetched automatically on movie discovery and refreshed every 30 days; exposed via `GET /api/v1/movies/{id}/profile`
- Full-text search across movies and people via `GET /api/v1/search` — free-text query plus structured filters (genre, year, person, department, language); backed by SQLite FTS5 or PostgreSQL tsvector + GIN indexes
- People as first-class entities — browse by person via `GET /api/v1/people/{id}` and full credit history via `GET /api/v1/people/{id}/credits`; index populated automatically during TMDb enrichment
- RSS/Atom feed for public subscription (global and per-user)
- JWT authentication via cookie (HTML) or Bearer token (REST API)
- ActivityPub federation — follow/unfollow remote users, accept/reject/remove followers, federated reviews broadcast as `Note` objects with `#MoviesDiary` + `#MovieTitle` hashtags, paginated outbox, boost/Announce tracking, NodeInfo discovery endpoint, shared inbox delivery, actor profile sync (bio, avatar, discoverable)
- Federation moderation — instance-level domain blocking (admin-managed), per-user actor blocking with `Block` activity, delivery filter excludes blocked actors and blocked-domain inboxes
- Watchlist — add movies to watch later, per-user; federated watchlist entries visible for remote actors
- User profiles — display name, bio, avatar, banner, custom profile fields; editable via HTML settings page or REST API
- Jellyfin/Plex auto-import — media server sends a webhook on playback stop, movies land in a watch queue; review and confirm with a rating to create diary entries; per-user webhook tokens with SHA-256 auth; setup UI at `/settings/integrations`
- Annual Wrap-Up — Spotify Wrapped for movies: per-user and instance-wide year-in-review with stats (top directors, actors, genres, rating distribution, watch time, rewatches, budget analysis), shareable HTML page at `/wrapups/{user_id}/{year}`, downloadable MP4 video with branded slides; admin-triggered or auto-generated in January
- CSV and JSON diary export
- File importer: upload CSV, TSV, JSON, or XLSX from any source (Letterboxd, IMDb, etc.), map columns to domain fields via a step-by-step wizard or REST API, save mapping profiles for repeat imports
- REST API v1 (`/api/v1/`) with full feature parity with the HTML interface
- OpenAPI documentation at `/docs` (Swagger UI) and `/scalar` (Scalar)
- CSRF protection on all HTML form routes (double-submit cookie, defense-in-depth on top of `SameSite=Strict`)
- Per-IP rate limiting via token bucket (production-grade, backed by `axum-governor`)
- Single-page app at `/app/` — React + TanStack Router + shadcn/ui, built with Vite, served from the backend with client-side routing fallback
- Terminal UI client (`crates/tui`) for logging reviews, bulk CSV import, and diary browsing

## Architecture

Hexagonal (Ports & Adapters) with Domain-Driven Design:

```
api-types           — shared REST API request/response DTOs (Serialize/Deserialize + utoipa schemas); used by presentation and tui
domain              — pure types and trait definitions, no external deps
application         — use cases (commands + queries), business logic orchestration; handlers delegate here for all domain logic
presentation        — Axum HTTP router, OpenAPI spec assembly, Swagger UI + Scalar serving, composition root for the HTTP process
worker              — standalone worker binary (event consumer, poster sync, federation)
adapters/
  auth                 — JWT issuance and validation (Argon2 passwords)
  sqlite               — SQLite repository + connection factory
  postgres             — PostgreSQL repository + connection factory
  metadata             — TMDB / OMDb HTTP client
  poster-fetcher       — downloads poster images
  image-storage        — stores images (posters + user avatars) on local filesystem or S3-compatible storage
  poster-sync          — event handler: triggers poster fetch+store on MovieDiscovered
  image-converter      — optional background worker: converts stored images to AVIF or WebP; backfills existing images via a 24h periodic job
  tmdb-enrichment      — event handler: fetches full movie profile (cast, crew, genres, keywords, box office) from TMDb on MovieEnrichmentRequested; resolves IMDb IDs automatically
  template-askama      — Askama HTML rendering
  rss                  — RSS/Atom feed generation
  export               — CSV and JSON diary serialization
  importer             — CSV/TSV/JSON/XLSX parser and column mapper for bulk import
  jellyfin             — Jellyfin webhook payload parser (MediaServerParser adapter)
  plex                 — Plex webhook payload parser (MediaServerParser adapter; requires Plex Pass)
  event-payload        — shared event serialization DTOs (used by all event bus adapters)
  sqlite-event-queue   — durable polling event queue backed by SQLite
  postgres-event-queue — durable polling event queue backed by PostgreSQL
  nats                 — NATS Core / JetStream event publisher and consumer
  event-publisher      — in-memory event channel (used in tests)
  wrapup-renderer      — annual wrap-up video generator (slide compositing via image/ab_glyph, stitching via ffmpeg)
  activitypub          — ActivityPub federation adapter (follow, inbox/outbox, actor); delegates to k-ap for protocol internals
  sqlite-search        — SQLite FTS5 implementation of SearchPort + SearchCommand
  postgres-search      — PostgreSQL tsvector + GIN implementation of SearchPort + SearchCommand
  sqlite-federation    — SQLite-backed federation repository
  postgres-federation  — PostgreSQL-backed federation repository
tui                 — terminal UI client (ratatui); shares api-types with presentation for typed API access
spa/                — React SPA (TanStack Router + shadcn/ui + Vite); served at /app/ by the backend
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

# TMDb metadata + enrichment (optional — enables full cast/crew/genre data)
# TMDB_API_KEY=your-key

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

# Image conversion (optional — converts stored images to AVIF or WebP to save space)
# IMAGE_CONVERSION_ENABLED=false
# IMAGE_CONVERSION_FORMAT=avif   # avif or webp

# Annual Wrap-Up video (optional — requires ffmpeg)
# WRAPUP_FONT_PATH=/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf
# WRAPUP_LOGO_PATH=./static/logo.webp   # watermark on video slides
# WRAPUP_BG_DIR=./static/wrapup-backgrounds   # slide background images (jpg/png/webp)
# FFMPEG_PATH=ffmpeg
# WRAPUP_MAX_CONCURRENT=2               # max parallel video renders

# Optional
HOST=0.0.0.0
PORT=3000
RATE_LIMIT=60           # requests per minute per IP (default: 60)
ALLOW_REGISTRATION=true # set to false to disable new sign-ups
SECURE_COOKIES=true     # set when serving over HTTPS
RUST_LOG=presentation=info,tower_http=info,worker=info,application=info

# CORS — comma-separated origins for SPA dev (omit or "*" for any)
# CORS_ORIGINS=http://localhost:5173

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

## SPA

The single-page app lives in `spa/` and is served at `/app/` by the backend. For local development:

```bash
cd spa && npm install && npm run dev   # http://localhost:5173/app/
```

Set `CORS_ORIGINS=http://localhost:5173` in the backend `.env` to allow cross-origin API calls during development.

For production, `npm run build` outputs to `spa/dist/` which the backend serves statically (included in Docker image automatically).

## Terminal UI

```bash
cargo run -p tui
```

Supports review logging, bulk CSV import (column order matches the export format), and diary browsing with review history.

## Development

A `Makefile` wraps the most common dev commands:

```bash
make           # default: fmt-check + clippy + test (same order as CI)
make fix       # auto-apply fmt + clippy fixes
make fmt       # apply rustfmt
make clippy    # clippy with -D warnings
make test      # cargo test
```

## Test

```bash
cargo test           # full workspace (requires DATABASE_URL for sqlx offline checks)
cargo test -p application   # domain-level unit tests only — no database required
```

The `application` crate has unit tests for core use cases backed by in-memory fakes from `domain`'s `test-helpers` feature. These run without a database and are the fastest feedback loop for business logic changes.

## Docker

The image contains both `presentation` (HTTP server) and `worker` (event processor), plus `ffmpeg` and DejaVu fonts for wrap-up video generation. Run them as separate containers sharing the same data volume:

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

## Media Server Integration

Auto-log movies you finish watching. Go to `/settings/integrations` to generate a webhook token, then configure your media server.

### Jellyfin

1. Install the **Webhook** plugin (Dashboard > Plugins > Catalog)
2. Add a **Generic** destination:
   - **URL**: `https://yourdomain.example.com/api/v1/webhooks/jellyfin`
   - **Header**: `Authorization` = `Bearer <your-token>`
   - **Send All Properties**: enabled
   - **Notification Type**: Playback Stop only
   - **Item Type**: Movies only

### Plex (requires Plex Pass)

1. Go to Settings > Webhooks in your Plex server
2. Add webhook URL: `https://yourdomain.example.com/api/v1/webhooks/plex`
3. Plex does not support custom headers natively — pass the token as a query param: `https://yourdomain.example.com/api/v1/webhooks/plex?token=<your-token>`

Movies you finish watching appear in your watch queue at `/watch-queue` — rate and confirm to add to your diary.

## Annual Wrap-Up

Generate a year-in-review summary for any user — top directors, actors, genres, rating distribution, total watch time, rewatch stats, and more. Available as a shareable HTML page and downloadable MP4 video.

**Generate via API** (admin only):
```bash
curl -X POST http://localhost:3000/api/v1/wrapups/generate \
  -H "Authorization: Bearer <admin-token>" \
  -H "Content-Type: application/json" \
  -d '{"user_id": "<uuid>", "start_date": "2025-01-01", "end_date": "2026-01-01"}'
```

Omit `user_id` for a global instance wrap-up. The worker computes stats in the background — poll `GET /api/v1/wrapups/{id}` for status.

**View:** `http://localhost:3000/wrapups/{user_id}/2025` (public, no login required)

**Auto-generate:** The worker runs a daily job in January that generates wrap-ups for all users with reviews in the previous year.

**Video:** Requires `ffmpeg` installed. Set `WRAPUP_FONT_PATH` and `WRAPUP_LOGO_PATH` for branded slides. Set `WRAPUP_BG_DIR` to a directory of background images for frutiger aero-style glass-panel slides. Cast profile photos and movie posters are embedded automatically. Download via `GET /api/v1/wrapups/{id}/video`.

## License

MIT License. See [LICENSE](LICENSE).
