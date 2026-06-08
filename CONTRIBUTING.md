# Contributing

Thanks for your interest in Movies Diary! This is a personal project but contributions are welcome — bug fixes, new features, docs improvements, or picking up the deprecated TUI.

## Getting started

1. Fork and clone the repo
2. Copy `.env.example` to `.env` and fill in at least `JWT_SECRET` and `OMDB_API_KEY`
3. Install Rust (stable, 2024 edition) and Node.js (for the SPA)
4. Run the backend and worker:

```bash
cargo run -p presentation   # HTTP server on :3000
cargo run -p worker          # event worker (separate terminal)
```

5. Run the SPA dev server:

```bash
cd spa && npm install && npm run dev
```

## Before submitting a PR

```bash
make           # runs fmt-check + clippy + test
```

Or individually:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cd spa && npx tsc --noEmit
```

All four must pass. PRs with clippy warnings or failing tests won't be merged.

## Architecture

The project follows hexagonal (ports & adapters) architecture. See `architecture.mmd` for the full diagram.

**Key rule:** presentation handlers never touch repositories directly — all domain logic goes through use cases in the `application` crate.

```
domain         → pure types, traits (ports), zero deps
application    → use cases, orchestration
presentation   → Axum handlers, routes, OpenAPI
worker         → event consumer, background jobs
adapters/*     → implements domain ports (sqlite, postgres, AP, etc.)
spa/           → React SPA (TanStack Router + shadcn/ui)
```

### Adding a new feature

1. **Domain first** — models in `domain/src/models/`, ports in `ports.rs`, events in `events.rs`
2. **Adapters** — implement ports in both `sqlite` and `postgres` adapters, add migration
3. **Application** — use cases in `application/src/<domain>/`, wire into `context.rs`
4. **API types** — DTOs in `api-types/src/`
5. **Presentation** — handler file in `handlers/<domain>.rs`, routes in `routes.rs`
6. **SPA** — API client in `spa/src/lib/api/`, hook in `spa/src/hooks/`, components
7. **Classic HTML** — Askama template + CSS in `static/style.css`

### Database adapters

Both SQLite and PostgreSQL are supported. If you add a migration or repository, implement it for both. The postgres adapter uses `$1, $2` params and `TIMESTAMPTZ`; SQLite uses `?` and text datetimes.

### Federation (ActivityPub)

Federation is feature-gated (`#[cfg(feature = "federation")]`). If your feature should federate, add domain events, handle them in `activitypub/src/event_handler.rs`, and create an AP object + inbound handler.

## Code style

- No comments unless the *why* is non-obvious
- Concise commit messages
- One feature per PR — don't bundle unrelated changes
- Follow existing patterns (check a similar feature for reference)

## Areas seeking help

- **TUI** (`crates/tui`) — deprecated, needs a maintainer to bring it up to feature parity
- **Tests** — integration tests for newer features (goals, watchlist, federation)
- **Docs** — API usage examples, deployment guides
