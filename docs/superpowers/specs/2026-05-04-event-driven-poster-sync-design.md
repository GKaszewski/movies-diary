# Event-Driven Poster Sync

**Date:** 2026-05-04
**Status:** Approved

## Problem

The `EventPublisher` infrastructure exists but only logs events via tracing. When a new movie is discovered (`MovieDiscovered` event), its poster should be automatically downloaded and stored — currently this requires a manual `POST /api/movies/{id}/sync-poster` call.

## Scope

- Introduce an `EventHandler` trait for composable event side-effects
- Implement `PosterSyncHandler` that reacts to `MovieDiscovered` by running the existing `sync_poster` use case with retry
- RSS feed is already generated fresh on every request — no event work needed there

## Design

### `EventHandler` trait (in `event-publisher` crate)

```rust
#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError>;
}
```

Lives in `event-publisher` crate alongside `ChannelEventPublisher`. Depends only on `domain` — no new crate dependencies required.

### `EventWorker` update

`EventWorker` gains a `handlers: Vec<Box<dyn EventHandler>>` field. On each received event:
1. Log the event via tracing (existing behavior, kept as default baseline)
2. Fan out to all handlers concurrently (or sequentially — see note below)
3. Handler errors are logged at ERROR level but do not stop the worker or other handlers

`create_event_channel` signature gains a `handlers` parameter:
```rust
pub fn create_event_channel(
    config: EventPublisherConfig,
    handlers: Vec<Box<dyn EventHandler>>,
) -> (ChannelEventPublisher, EventWorker)
```

**Fan-out strategy:** sequential for now (simpler, avoids concurrent mutation of shared state). If handler latency becomes a concern, switch to `tokio::join_all`.

### `PosterSyncHandler` (in `presentation` crate)

New file: `crates/presentation/src/event_handlers.rs`

```rust
pub struct PosterSyncHandler {
    ctx: AppContext,
    max_retries: u32,
}
```

Behavior per event:
- `MovieDiscovered { movie_id, external_metadata_id }` → build `SyncPosterCommand`, call `sync_poster::execute()` with exponential backoff
- All other events → `Ok(())` immediately

**Retry logic:** up to `max_retries` retries after the initial attempt (default: 3, so 4 total attempts), with exponential backoff delays 1s → 2s → 4s before each retry. After exhausting all attempts, logs at ERROR level and returns `Err`.

### Wiring (`main.rs`)

```rust
let poster_handler = PosterSyncHandler::new(app_ctx.clone(), 3); // 3 retries = 4 total attempts
let (event_publisher, event_worker) = create_event_channel(
    EventPublisherConfig::from_env(),
    vec![Box::new(poster_handler)],
);
tokio::spawn(event_worker.run());
```

`AppContext` is `Clone` (all fields are `Arc<dyn Trait>`), so cloning for the handler is cheap.

## Data Flow

```
POST /api/diary
  → log_review::execute()
      → movie not in DB → fetch metadata → MovieDiscovered published
      → review saved → ReviewLogged published
          ↓
        mpsc channel
          ↓
        EventWorker::run()
          → tracing log
          → PosterSyncHandler::handle(MovieDiscovered)
              → sync_poster::execute() [attempt 1]
              → on failure: sleep 1s → attempt 2
              → on failure: sleep 2s → attempt 3
              → on failure: log ERROR, done
```

## Files Changed

| File | Change |
|---|---|
| `crates/adapters/event-publisher/src/lib.rs` | Add `EventHandler` trait; update `EventWorker` and `create_event_channel` |
| `crates/presentation/src/event_handlers.rs` | New — `PosterSyncHandler` |
| `crates/presentation/src/main.rs` | Wire `PosterSyncHandler` into `create_event_channel` |

No new crate dependencies. No changes to domain or application layers.

## Verification

```bash
cargo build                        # full workspace clean build
cargo test                         # existing tests still pass

# Manual: log a review for a new movie
# → check logs for "event: movie_discovered"
# → check logs for poster sync attempt
# → check object store / storage for saved poster file
```
