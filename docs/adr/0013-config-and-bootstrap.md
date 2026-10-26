# Typed configuration crate and a bootstrap crate that owns all wiring

Configuration was read ad hoc from `std::env` in many adapters, and the two binaries each carried
hundreds of lines of adapter wiring (`wire_dependencies`, `create_event_publisher`, the worker's
handler and job assembly). ADR-0005 made `composition` the single root for deps, but the binaries
still named adapters themselves, and `infra-wiring` existed only so `application` could re-export an
`AppConfig`.

## Decision

**`config` crate.** One typed `config::AppConfig` with sections (`server`, `database`, `auth`,
`events`, `nats`, `storage`, `image_conversion`, `metadata`, `federation`, `wrapup`), loaded once by
the binary via `config::load()`. Layering, lowest to highest precedence:

1. built-in defaults;
2. `config.toml`, or the file named by `CONFIG_FILE` (a named file that is missing is an error);
3. environment variables, using the legacy names (`DATABASE_URL`, `PORT`, `JWT_SECRET`, ...) with no
   prefix. Every variable that ever worked keeps its name, meaning and default.

A malformed value fails startup with an error naming the variable (`PORT=abc`). An empty or
whitespace-only value is treated as unset, so the default applies (or `None` for optional fields)
rather than being an error. Adapters take their section (or a typed config built from it), never
raw environment strings.

**`bootstrap` crate.** Replaces `composition` and `infra-wiring`. It is the only crate that names
concrete adapters. It exposes `build_server(&AppConfig) -> ServerApp { router, addr }`,
`build_worker(&AppConfig) -> WorkerApp { service }`, `build_server_on(&AppConfig, SqlitePool)` (the
same router on a caller-provided pool, for tests) and `init_tracing`. Bootstrap runs
on `SqlitePool`; the event-bus backend is chosen inside bootstrap and not re-exported. `application`
keeps small application-owned `AppSettings` / `WrapUpSettings` types built from the `config`
sections, so use-case deps structs and their test fixtures
are unchanged, and `application` no longer pulls `sqlx` or `anyhow`.

**Binaries are thin.** `server` and `worker` do dotenv, `config::load`, `init_tracing`, one
`bootstrap::build_*` call and `axum::serve` / `service.run`. Their `[dependencies]` are limited to
`bootstrap`, `config`, `tokio`, `axum`, `anyhow`, `dotenvy` and `tracing`; `make
check-binaries-are-thin` enforces it. The `federation` and `nats` features forward to bootstrap.

## Consequences

- Adding or changing an adapter touches `bootstrap` only; the binaries do not change.
- The wiring is reachable from tests (`build_server_on`) without a process or a port.
- `check-federation-off-social` now inspects `crates/bootstrap/src/federation.rs`: federation-off
  must still wire `LocalSocialService`, never social noops (ADR-0009).
- Operators can move settings into `config.toml` incrementally; existing `.env` files keep working.
