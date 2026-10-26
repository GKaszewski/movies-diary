# Code Style Guide

This document defines the coding standards for this project. All contributors (human and agent) must follow these rules. When in doubt, prefer clarity and correctness over cleverness and speed.

## Philosophy

- Code reads like prose
- Do things properly — no shortcuts
- SOLID principles — always, no exceptions
- KISS — simplest solution that solves the problem **correctly and properly**. KISS means no unnecessary complexity, NOT skipping proper architecture. If the design calls for a trait, a newtype, or a pattern — that IS the simple solution. When in doubt, follow the architecture (CONTEXT.md, ADRs) over perceived simplicity.
- DRY — one source of truth for every piece of knowledge
- Use design patterns where they fit (Strategy, Observer, Builder, etc.) — name them in code so intent is clear
- Single responsibility everywhere: files, functions, types, modules
- Single point of entry, single control flow — no parallel data/control paths
- Open/Closed — extend via new types, don't modify existing ones
- Depend on abstractions (traits), not concretions
- Keep things as private as possible
- Zero magic numbers, strings, or unnamed constants
- Newtypes for self-documenting code
- Descriptive variable names — `review_rating`, not `rr`. `watched_on`, not `wd`. `poster_url`, not `pu`. No abbreviations unless universally understood (e.g., `id`, `url`, `i` for a trivial loop index).

## Project Structure

### Module Organization

Deep module hierarchy, grouped by concern. No flat file dumps. Each file is its own small, enclosed thing.

```
crates/domain/src/
├── models/
│   ├── review.rs
│   ├── diary_entry.rs
│   └── movie.rs
├── value_objects/
│   ├── movie_id.rs
│   ├── rating.rs
│   └── watched_on.rs
├── ports/
│   ├── review_repository.rs
│   └── event_publisher.rs
└── ...
```

### File Size

Small files with a single concern. If a file grows past ~100-150 lines, it's probably doing too much — split it.

### Tests

Tests live outside production files, apart from the few exceptions listed below. Each crate keeps its tests in `crates/<crate>/tests/`, one file per unit under test, grouped by the same module as the production code. Cargo builds each top-level `tests/<group>.rs` as one integration-test binary; the group file lists its members with `#[path]`:

```
crates/application/src/diary/log_review.rs
crates/application/tests/diary.rs                  # mod support; + #[path] list of the group
crates/application/tests/diary/log_review_test.rs
crates/application/tests/support/                  # shared contexts and command builders
```

Adapters follow the same layout (`crates/adapters/sqlite/tests/repositories/diary_repository_test.rs`). Route smoke, API and handler tests run against the real composition and live in `crates/bootstrap/tests/`; http-axum also has integration tests in `crates/adapters/http-axum/tests/` (`log_review_form_test.rs`, `openapi_contract_test.rs`).

Inline `#[cfg(test)]` tests remain only where they must reach private items:

- `crates/adapters/http-axum`: the `api/social`, `federation/blocklist_api` and `html/social` mapper tests (`tests.rs` beside each; shared fixtures in `src/test_support.rs`) and the rate-limit tests in `router.rs`
- `crates/application/src/users/diary_filter.rs`
- `crates/adapters/nats/src/subject.rs`
- `crates/tui/src/client.rs`

## Naming

### Conventions

- Files and modules: `snake_case`
- Types: `PascalCase` with semantic suffixes for clarity and navigation
- Functions and methods: `snake_case`, verb-first, descriptive

### Type Suffixes

| Suffix | Usage | Example |
|---|---|---|
| (none) | Value objects, entities | `Review`, `Movie`, `DiaryEntry` |
| `Id` | Identity newtypes | `MovieId`, `ReviewId`, `UserId` |
| `Config` | Configuration data | `ServerConfig`, `StorageConfig` |
| `Service` | Domain services | `LocalSocialService` |
| `Error` | Error types | `DomainError`, `AuthError` |
| `UseCase` | Application use cases | `LogReviewUseCase`, `GetDiaryUseCase` |
| `Strategy` | Strategy trait/impls | `RatingAggregationStrategy` |

Ports are named for the capability they provide (`ReviewRepository`, `EventPublisher`); their implementations are named for the technology (`SqliteReviewRepository`).

### Newtypes

Use newtypes everywhere a primitive carries domain meaning:

```rust
// YES
struct MovieId(Uuid);
struct ReviewId(Uuid);
struct Rating(u8);
struct PosterUrl(Url);

// NO
type MovieId = String;
fn log_review(movie: &str, rating: u8, watched_on: &str) -> ...
```

Newtypes are self-documenting and prevent accidental misuse (can't pass a `MovieId` where a `ReviewId` is expected).

## Functions and Methods

### Single Responsibility

Each function does ONE thing. Compose small functions into larger behaviors.

```rust
// YES
fn resolve_movie(...) -> Movie { ... }
fn build_review(movie: &Movie, draft: ReviewDraft) -> Review { ... }
fn record_diary_entry(review: &Review) -> DiaryEntry { ... }

// NO
fn log_review(...) -> DiaryEntry {
    // 80 lines doing movie resolution, review construction and diary recording
}
```

### Visibility

Default to private. Only `pub` what the module's consumers actually need. Use `pub(crate)` for intra-crate sharing that shouldn't leak outside.

### impl Blocks

Split by concern:

```rust
// Construction
impl Review {
    pub fn new(movie: MovieId, rating: Rating, watched_on: WatchedOn) -> Self { ... }
}

// Queries
impl Review {
    pub fn is_rewatch(&self) -> bool { ... }
    pub fn rating(&self) -> Rating { ... }
}

// Mutations
impl Review {
    pub fn revise_rating(&mut self, rating: Rating) { ... }
}
```

## Error Handling

### Domain and Application: `thiserror`

Typed, matchable error enums. Every error variant is meaningful.

```rust
#[derive(Debug, thiserror::Error)]
pub enum DiaryError {
    #[error("movie {0} not found")]
    MovieNotFound(MovieId),
    #[error("review rejected: {0}")]
    ReviewRejected(String),
}
```

### Adapters and Bootstrap: `anyhow`

Pragmatic error handling. Context strings for debugging.

```rust
fn fetch_poster_blob(poster_url: &PosterUrl) -> anyhow::Result<Bytes> {
    let data = http_client.get(poster_url)
        .context("failed to fetch poster")?;
    // ...
}
```

### Never `unwrap` or `expect` in production code

Errors propagate with `?` and bubble up to a single handler. No scattered panic points.

`unwrap` is allowed **only** in test code.

## Configuration

Secrets (API keys, JWT secrets) and deployment settings are read once at startup by the `config` crate into a typed `AppConfig`: built-in defaults, then `config.toml` (or the file named by `CONFIG_FILE`), then environment variables. Malformed values fail startup; empty values count as unset. Only the binaries load configuration; everything else receives typed config values.

Zero magic numbers or strings. Everything named, everything explained by its name.

```rust
// YES
const MAX_REVIEW_BODY_LENGTH: usize = 10_000;

// NO
if body.len() > 10_000 { ... }
```

Constants are only for true invariants that never change. Tunable values belong in config.

## Comments

Comments only where a name, test or ADR cannot carry the reason. A comment is noise that drifts out of date while the code moves on, so prefer to put the reason somewhere that cannot rot silently:

- **A name.** Rename the function, the variable, or the type until the reason is visible in the code.
- **A test.** A constraint worth a comment is worth a test that fails when someone breaks it. `media_is_resolved_before_the_cascade_runs` outlives any note explaining why the order matters.
- **An ADR.** Architectural reasoning belongs in `docs/adr/`, where it is versioned and discoverable.

When none of those can carry it, a short comment stating the reason (never restating the code) is acceptable.

## Dependencies

- Use good crates when they exist — don't reinvent
- Prefer crates with minimal transitive dependencies
- Aim for small memory footprint and executable size
- Every dependency that has a `[workspace.dependencies]` entry is declared `workspace = true`; crates inherit `edition` from `[workspace.package]`
- Domain crate: zero external dependencies beyond `thiserror`, `serde` and small value-type crates
- Application crate: depends only on domain
- Adapters: free to pull in platform crates (`axum`, `sqlx`, `nats`, etc.)
- Bootstrap: wiring only; the only crate that names adapters
- Binaries (`server`, `worker`): load config, call `bootstrap`, run — nothing else (`make check-binaries-are-thin`)

## Trait Objects for Ports

Ports are `dyn` behind `Arc`. Use cases hold their dependencies as trait objects:

```rust
pub struct Deps {
    pub reviews: Arc<dyn ReviewRepository>,
    pub events: Arc<dyn EventPublisher>,
}
```

Prefer a closed enum over `dyn` only where exhaustiveness is the point — when adding a variant must force every consumer to handle it. That is a decision about compiler-enforced coverage, not about dispatch cost.

## Testing

### Test Helpers Are Gated

`domain::testing` (factories, the cascading `InMemoryStore`, noops and `Refusing*` fakes) compiles only with the `test-helpers` feature, which crates enable in their `[dev-dependencies]`. Production builds must never contain fakes. Build entities with the `domain::testing` factories instead of repeating struct literals in each test.

### Coverage Targets

| Crate | Approach | Target |
|---|---|---|
| Domain | TDD (red-green-refactor) | 100% |
| Application | TDD | High coverage |
| Adapters | Integration tests where practical | Best effort |
| Bootstrap | Route smoke, API and handler tests over the real wiring | Every route reachable |

### Fakes Must Not Be Kinder Than Production

A fake that is more forgiving than the real adapter turns a test into a lie: it passes whichever way the code is written, which is worse than having no test. Two rules follow.

**A fake must reproduce the constraints the database enforces.** SQLite removes dependent rows through `ON DELETE CASCADE`, so an in-memory store must cascade the same way, and enforce the same uniqueness constraints. Without that, an ordering requirement such as "resolve the poster before deleting the `DiaryEntry`" could not be tested at all.

**A fake must be able to fail.** Every port whose failure is meant to degrade rather than propagate needs a fake that refuses (a poster fetcher that errors, an event publisher that rejects). A best-effort path with no failing test is a path that has never run.

When a claim cannot be observed through the fake, move the test to where the real thing runs rather than asserting it against the fake's own behaviour.

### Test Naming

Tests read as specifications:

```rust
#[tokio::test]
async fn logging_review_creates_diary_entry() { ... }

#[tokio::test]
async fn rewatch_review_keeps_original_entry() { ... }
```

## Control Flow

### Single Point of Entry

One path through the code. No parallel flows that do similar things in different places.

```rust
// YES — one function handles movie resolution
let movie = movie_resolver.resolve(movie_id, requester);

// NO — movie resolution duplicated in two places
// diary_handler.rs: resolves movie
// import_handler.rs: also resolves movie with slightly different logic
```

### Single Call, Cascading Changes

When something changes, it should cascade from one point. Don't require updating multiple files for a single logical change.

### No God Objects

If a struct has more than 5-6 fields, question whether it's doing too much. If a function takes more than 4-5 parameters, consider grouping them into a dedicated type.
