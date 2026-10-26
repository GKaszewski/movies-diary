# Contributing

Thanks for your interest in Movies Diary! This is a personal project but contributions are welcome — bug fixes, new features, docs improvements, or picking up the deprecated TUI.

## Getting started

1. Fork and clone the repo
2. Copy `.env.example` to `.env` and fill in at least `JWT_SECRET` and `OMDB_API_KEY`
3. Install Rust (stable, 2024 edition) and Bun (for the SPA)
4. Run the backend and worker:

```bash
cargo run -p server         # HTTP server on :3000
cargo run -p worker          # event worker (separate terminal)
```

5. Run the SPA dev server:

```bash
cd spa && bun install && bun run dev
```

## Before submitting a PR

```bash
make check     # the full gate; plain `make` runs it too
```

`make check` runs, in order: `fmt-check`, `clippy` (workspace, all targets, `-D warnings`), `test`, `test-federation-off` (workspace tests with default features off), then the layering guards (`check-appcontext`, `check-handler-deps`, `check-handler-repos`, `check-handler-services`, `check-http-adapter-deps`, `check-federation-off-social`, `check-sqlite-is-ap-free`, `check-binaries-are-thin`, `check-federation-off-deps`, `check-nats-isolated`, `check-no-second-web-stack`). CI runs the same steps.

For the SPA also run:

```bash
cd spa && bunx tsc --noEmit
```

PRs with a failing gate won't be merged.

## Architecture

The project follows hexagonal (ports & adapters) architecture. See `architecture.mmd` for the full diagram.

**Key rules:**
- HTTP handlers never touch repositories directly — all domain logic goes through use cases in the `application` crate
- Application use cases return raw domain data — URL formatting, date display, and view model assembly belong in the http-axum mappers (`crates/adapters/http-axum/src/html/mappers/` for the classic UI, the DTO conversions beside each handler under `api/`)
- Use cases called from HTTP handlers take a `&FooDeps` struct (registered in `application::Deps`, built by `bootstrap`) — never `&AppContext` itself, which `application` cannot even depend on. A few keep individual `Arc<dyn Trait>` params instead: `enrich_movie` and `request_enrichment` because they are called from adapters rather than handlers, and `diary::log_review` because its one extra dependency comes from `Services` rather than a repository. `wrapup::compute`, `import::cleanup` and `integrations::cleanup` also take individual params, but they are only ever called from jobs. See ADR-0007 for the exact list and why each is legitimate

```
domain         → pure types, traits (ports), zero deps
application    → use cases, orchestration
config         → typed AppConfig: defaults → config.toml / CONFIG_FILE → env
bootstrap      → composition root, the only crate that names adapters
server, worker → thin binaries: load config, call bootstrap, run
adapters/*     → implement domain ports (sqlite, activitypub, ...); http-axum is the HTTP driving adapter
spa/           → React SPA (TanStack Router + shadcn/ui)
```

Cargo features: `federation` (default; ActivityPub) and `nats`. `sqlite` and `sqlite-federation` remain as deprecated aliases so old build commands keep working.

**Dependencies.** Every external crate is declared once in the root `[workspace.dependencies]`, usually with `default-features = false`; member crates write `foo = { workspace = true, features = [...] }` and enable only the features they use. A new crate goes into both `members` and `default-members` in the root `Cargo.toml` (only `tui` is left out of `default-members`). `cargo machete` should stay clean, and each crate must compile on its own (`cargo check -p <crate> --all-targets`), not just through feature unification in the binaries.

### Adding a new feature

1. **Domain first** — models in `domain/src/models/`, ports in `domain/src/ports/`, events in `domain/src/events.rs`
2. **Adapters** — implement ports under `crates/adapters/sqlite/src/repositories/<aggregate>/`, add a migration
3. **Application** — use cases in `application/src/<domain>/`, add the deps struct to `application/src/deps.rs`, build it in `bootstrap`
4. **API types** — DTOs in `api-types/src/`
5. **HTTP** — JSON handlers in `crates/adapters/http-axum/src/api/<domain>.rs`, classic HTML handlers in `.../html/<domain>.rs` (templates in `html/templates/`), routes in `router.rs` / `api/mod.rs` / `html/mod.rs`
6. **SPA** — API client in `spa/src/lib/api/`, hook in `spa/src/hooks/`, components
7. **Classic HTML** — Askama template + CSS in `static/style.css`

### Tests

Tests live in `crates/<crate>/tests/`, not in the production file. A few inline tests remain where they need private items; `CODE_STYLE.md` lists them. Build fixtures with the `domain::testing` factories and use the cascading `InMemoryStore` and `Refusing*` fakes. Route smoke, API and handler tests over the real wiring are in `crates/bootstrap/tests/`.

### Database adapters

SQLite is the only supported backend (see [ADR-0012](docs/adr/0012-sqlite-is-the-only-backend.md)). Add migrations under `crates/adapters/sqlite`; SQLite uses `?` params and text datetimes.

### Federation (ActivityPub)

Events: never publish from a use case after the fact. Pass the events to the repository/command-port method that writes the data so they land in the outbox (`event_queue`) in the same transaction (ADR-0015). The worker delivers them at-least-once and a ledger skips handlers already done, so every handler must be idempotent.

Federation is feature-gated (`#[cfg(feature = "federation")]`). If your feature should federate, add domain events, handle them in `crates/adapters/activitypub/src/event_handler.rs`, and create an AP object + inbound handler.

## Code style

- No comments unless the *why* is non-obvious
- Concise commit messages
- One feature per PR — don't bundle unrelated changes
- Follow existing patterns (check a similar feature for reference)

## Areas seeking help

- **TUI** (`crates/tui`) — deprecated, needs a maintainer to bring it up to feature parity
- **Tests** — the domain and application crates have 400+ tests; more route and handler tests in `crates/bootstrap/tests/` are welcome
- **Docs** — API usage examples, deployment guides
