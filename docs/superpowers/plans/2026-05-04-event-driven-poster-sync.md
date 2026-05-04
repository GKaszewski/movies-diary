# Event-Driven Poster Sync Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an `EventHandler` trait to the event-publisher adapter and implement `PosterSyncHandler` so that a `MovieDiscovered` event automatically triggers the existing `sync_poster` use case with exponential-backoff retry.

**Architecture:** `EventWorker` gains a `Vec<Box<dyn EventHandler>>` and fans out each received event to all registered handlers sequentially. `PosterSyncHandler` lives in the `presentation` crate (composition root), holds `AppContext`, and calls `sync_poster::execute` on `MovieDiscovered` events — ignoring all others. Retry is up to 3 retries (4 total attempts) with delays 1s → 2s → 4s.

**Tech Stack:** Rust, tokio::sync::mpsc, async-trait, existing `sync_poster` use case

---

## File Map

| File | Status | Responsibility |
|---|---|---|
| `crates/adapters/event-publisher/src/lib.rs` | Modify | Add `EventHandler` trait; extend `EventWorker` and `create_event_channel` |
| `crates/application/src/commands.rs` | Modify | Add `#[derive(Clone)]` to `SyncPosterCommand` |
| `crates/presentation/src/lib.rs` | Modify | Expose `pub mod event_handlers` |
| `crates/presentation/src/event_handlers.rs` | Create | `PosterSyncHandler` implementation |
| `crates/presentation/src/main.rs` | Modify | Wire `PosterSyncHandler` into `create_event_channel` |

---

## Task 1: Add `EventHandler` trait and update `EventWorker`

**Files:**
- Modify: `crates/adapters/event-publisher/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Add to the bottom of `crates/adapters/event-publisher/src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use async_trait::async_trait;
    use domain::{
        errors::DomainError,
        events::DomainEvent,
        value_objects::{ExternalMetadataId, MovieId},
    };

    struct RecordingHandler {
        calls: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl EventHandler for RecordingHandler {
        async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError> {
            let label = match event {
                DomainEvent::MovieDiscovered { .. } => "movie_discovered",
                DomainEvent::ReviewLogged { .. } => "review_logged",
            };
            self.calls.lock().unwrap().push(label.to_string());
            Ok(())
        }
    }

    #[tokio::test]
    async fn single_handler_receives_event() {
        let calls = Arc::new(Mutex::new(vec![]));
        let handler = RecordingHandler { calls: Arc::clone(&calls) };
        let config = EventPublisherConfig { channel_buffer: 8 };
        let (publisher, worker) = create_event_channel(config, vec![Box::new(handler)]);

        tokio::spawn(worker.run());

        let event = DomainEvent::MovieDiscovered {
            movie_id: MovieId::generate(),
            external_metadata_id: ExternalMetadataId::new("tt1234567".into()).unwrap(),
        };
        publisher.publish(&event).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        assert_eq!(*calls.lock().unwrap(), vec!["movie_discovered"]);
    }

    #[tokio::test]
    async fn multiple_handlers_all_receive_event() {
        let calls1 = Arc::new(Mutex::new(vec![]));
        let calls2 = Arc::new(Mutex::new(vec![]));
        let handler1 = RecordingHandler { calls: Arc::clone(&calls1) };
        let handler2 = RecordingHandler { calls: Arc::clone(&calls2) };
        let config = EventPublisherConfig { channel_buffer: 8 };
        let (publisher, worker) = create_event_channel(
            config,
            vec![Box::new(handler1), Box::new(handler2)],
        );

        tokio::spawn(worker.run());

        let event = DomainEvent::MovieDiscovered {
            movie_id: MovieId::generate(),
            external_metadata_id: ExternalMetadataId::new("tt9999999".into()).unwrap(),
        };
        publisher.publish(&event).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        assert_eq!(calls1.lock().unwrap().len(), 1);
        assert_eq!(calls2.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn handler_error_does_not_stop_worker() {
        struct FailingHandler;
        #[async_trait]
        impl EventHandler for FailingHandler {
            async fn handle(&self, _: &DomainEvent) -> Result<(), DomainError> {
                Err(DomainError::InfrastructureError("boom".into()))
            }
        }

        let calls = Arc::new(Mutex::new(vec![]));
        let good = RecordingHandler { calls: Arc::clone(&calls) };
        let config = EventPublisherConfig { channel_buffer: 8 };
        let (publisher, worker) = create_event_channel(
            config,
            vec![Box::new(FailingHandler), Box::new(good)],
        );

        tokio::spawn(worker.run());

        let event = DomainEvent::MovieDiscovered {
            movie_id: MovieId::generate(),
            external_metadata_id: ExternalMetadataId::new("tt0000001".into()).unwrap(),
        };
        publisher.publish(&event).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // good handler still ran despite failing handler before it
        assert_eq!(calls.lock().unwrap().len(), 1);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p event-publisher 2>&1 | tail -20
```

Expected: compile errors — `EventHandler` not defined, `create_event_channel` wrong arity.

- [ ] **Step 3: Replace `lib.rs` with updated implementation**

Replace the full content of `crates/adapters/event-publisher/src/lib.rs` with:

```rust
use async_trait::async_trait;
use domain::{errors::DomainError, events::DomainEvent, ports::EventPublisher};
use tokio::sync::mpsc;

pub struct EventPublisherConfig {
    pub channel_buffer: usize,
}

impl EventPublisherConfig {
    pub fn from_env() -> Self {
        let channel_buffer = std::env::var("EVENT_CHANNEL_BUFFER")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(128);
        Self { channel_buffer }
    }
}

#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError>;
}

pub struct ChannelEventPublisher {
    sender: mpsc::Sender<DomainEvent>,
}

#[async_trait]
impl EventPublisher for ChannelEventPublisher {
    async fn publish(&self, event: &DomainEvent) -> Result<(), DomainError> {
        self.sender
            .send(event.clone())
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))
    }
}

pub struct EventWorker {
    receiver: mpsc::Receiver<DomainEvent>,
    handlers: Vec<Box<dyn EventHandler>>,
}

impl EventWorker {
    pub async fn run(mut self) {
        while let Some(event) = self.receiver.recv().await {
            match &event {
                DomainEvent::ReviewLogged {
                    review_id,
                    movie_id,
                    user_id,
                    rating,
                    watched_at,
                } => {
                    tracing::info!(
                        review_id  = %review_id.value(),
                        movie_id   = %movie_id.value(),
                        user_id    = %user_id.value(),
                        rating     = rating.value(),
                        watched_at = %watched_at,
                        "event: review_logged"
                    );
                }
                DomainEvent::MovieDiscovered {
                    movie_id,
                    external_metadata_id,
                } => {
                    tracing::info!(
                        movie_id    = %movie_id.value(),
                        external_id = external_metadata_id.value(),
                        "event: movie_discovered"
                    );
                }
            }
            for handler in &self.handlers {
                if let Err(e) = handler.handle(&event).await {
                    tracing::error!("event handler error: {e}");
                }
            }
        }
        tracing::info!("event worker shut down");
    }
}

pub fn create_event_channel(
    config: EventPublisherConfig,
    handlers: Vec<Box<dyn EventHandler>>,
) -> (ChannelEventPublisher, EventWorker) {
    let (tx, rx) = mpsc::channel(config.channel_buffer);
    (
        ChannelEventPublisher { sender: tx },
        EventWorker {
            receiver: rx,
            handlers,
        },
    )
}

#[cfg(test)]
mod tests {
    // paste the test module from Step 1 here
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test -p event-publisher 2>&1 | tail -20
```

Expected: `test result: ok. 3 passed`

- [ ] **Step 5: Commit**

```bash
git add crates/adapters/event-publisher/src/lib.rs
git commit -m "feat(event-publisher): add EventHandler trait and fan-out in EventWorker"
```

---

## Task 2: Derive `Clone` on `SyncPosterCommand`

**Files:**
- Modify: `crates/application/src/commands.rs`

The `PosterSyncHandler` retry loop reconstructs the command on each attempt, which requires `Clone` on `String` (already impl'd) and `Uuid` (Copy) — but it's cleaner to `#[derive(Clone)]` directly.

- [ ] **Step 1: Add `#[derive(Clone)]` to `SyncPosterCommand`**

In `crates/application/src/commands.rs`, find the `SyncPosterCommand` struct (line ~17) and add the derive:

```rust
#[derive(Clone)]
pub struct SyncPosterCommand {
    pub movie_id: Uuid,
    pub external_metadata_id: String,
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo build -p application 2>&1 | tail -10
```

Expected: clean build.

- [ ] **Step 3: Commit**

```bash
git add crates/application/src/commands.rs
git commit -m "feat(application): derive Clone on SyncPosterCommand"
```

---

## Task 3: Implement `PosterSyncHandler`

**Files:**
- Create: `crates/presentation/src/event_handlers.rs`
- Modify: `crates/presentation/src/lib.rs`

- [ ] **Step 1: Write the failing test first — create `event_handlers.rs` with tests only**

Create `crates/presentation/src/event_handlers.rs`:

```rust
use std::time::Duration;

use application::{commands::SyncPosterCommand, context::AppContext, use_cases::sync_poster};
use async_trait::async_trait;
use domain::{errors::DomainError, events::DomainEvent};
use event_publisher::EventHandler;

pub struct PosterSyncHandler {
    ctx: AppContext,
    max_retries: u32,
}

impl PosterSyncHandler {
    pub fn new(ctx: AppContext, max_retries: u32) -> Self {
        Self { ctx, max_retries }
    }
}

#[async_trait]
impl EventHandler for PosterSyncHandler {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use async_trait::async_trait;
    use application::config::AppConfig;
    use domain::{
        errors::DomainError,
        events::DomainEvent,
        models::{DiaryEntry, DiaryFilter, Movie, Review, ReviewHistory, User, collections::Paginated},
        ports::{
            AuthService, EventPublisher, GeneratedToken, MetadataClient, MetadataSearchCriteria,
            MovieRepository, PasswordHasher, PosterFetcherClient, PosterStorage, UserRepository,
        },
        value_objects::{
            Email, ExternalMetadataId, MovieId, MovieTitle, PasswordHash, PosterPath, PosterUrl,
            Rating, ReleaseYear, ReviewId, UserId,
        },
    };

    // Panic stubs — never called in the "ignored event" test path
    struct PanicRepo;
    struct PanicMetadata;
    struct PanicFetcher;
    struct PanicStorage;
    struct PanicAuth;
    struct PanicHasher;
    struct PanicUserRepo;
    struct NoopPublisher;

    #[async_trait]
    impl MovieRepository for PanicRepo {
        async fn get_movie_by_external_id(&self, _: &ExternalMetadataId) -> Result<Option<Movie>, DomainError> { panic!("unexpected") }
        async fn get_movie_by_id(&self, _: &MovieId) -> Result<Option<Movie>, DomainError> { panic!("unexpected") }
        async fn get_movies_by_title_and_year(&self, _: &MovieTitle, _: &ReleaseYear) -> Result<Vec<Movie>, DomainError> { panic!("unexpected") }
        async fn upsert_movie(&self, _: &Movie) -> Result<(), DomainError> { panic!("unexpected") }
        async fn save_review(&self, _: &Review) -> Result<DomainEvent, DomainError> { panic!("unexpected") }
        async fn query_diary(&self, _: &DiaryFilter) -> Result<Paginated<DiaryEntry>, DomainError> { panic!("unexpected") }
        async fn get_review_history(&self, _: &MovieId) -> Result<ReviewHistory, DomainError> { panic!("unexpected") }
    }

    #[async_trait]
    impl MetadataClient for PanicMetadata {
        async fn fetch_movie_metadata(&self, _: &MetadataSearchCriteria) -> Result<Movie, DomainError> { panic!("unexpected") }
        async fn get_poster_url(&self, _: &ExternalMetadataId) -> Result<Option<PosterUrl>, DomainError> { panic!("unexpected") }
    }

    #[async_trait]
    impl PosterFetcherClient for PanicFetcher {
        async fn fetch_poster_bytes(&self, _: &PosterUrl) -> Result<Vec<u8>, DomainError> { panic!("unexpected") }
    }

    #[async_trait]
    impl PosterStorage for PanicStorage {
        async fn store_poster(&self, _: &MovieId, _: &[u8]) -> Result<PosterPath, DomainError> { panic!("unexpected") }
        async fn get_poster(&self, _: &PosterPath) -> Result<Vec<u8>, DomainError> { panic!("unexpected") }
    }

    #[async_trait]
    impl AuthService for PanicAuth {
        async fn generate_token(&self, _: &UserId) -> Result<GeneratedToken, DomainError> { panic!("unexpected") }
        async fn validate_token(&self, _: &str) -> Result<UserId, DomainError> { panic!("unexpected") }
    }

    #[async_trait]
    impl PasswordHasher for PanicHasher {
        async fn hash(&self, _: &str) -> Result<PasswordHash, DomainError> { panic!("unexpected") }
        async fn verify(&self, _: &str, _: &PasswordHash) -> Result<bool, DomainError> { panic!("unexpected") }
    }

    #[async_trait]
    impl UserRepository for PanicUserRepo {
        async fn find_by_email(&self, _: &Email) -> Result<Option<User>, DomainError> { panic!("unexpected") }
        async fn save(&self, _: &User) -> Result<(), DomainError> { panic!("unexpected") }
    }

    #[async_trait]
    impl EventPublisher for NoopPublisher {
        async fn publish(&self, _: &DomainEvent) -> Result<(), DomainError> { Ok(()) }
    }

    fn panic_ctx() -> AppContext {
        AppContext {
            repository: Arc::new(PanicRepo),
            metadata_client: Arc::new(PanicMetadata),
            poster_fetcher: Arc::new(PanicFetcher),
            poster_storage: Arc::new(PanicStorage),
            event_publisher: Arc::new(NoopPublisher),
            auth_service: Arc::new(PanicAuth),
            password_hasher: Arc::new(PanicHasher),
            user_repository: Arc::new(PanicUserRepo),
            config: AppConfig { allow_registration: false },
        }
    }

    #[tokio::test]
    async fn review_logged_is_ignored() {
        let handler = PosterSyncHandler::new(panic_ctx(), 3);
        let event = DomainEvent::ReviewLogged {
            review_id: ReviewId::generate(),
            movie_id: MovieId::generate(),
            user_id: UserId::generate(),
            rating: Rating::new(4).unwrap(),
            watched_at: chrono::NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
        };
        // returns Ok without touching any panic stubs
        assert!(handler.handle(&event).await.is_ok());
    }
}
```

- [ ] **Step 2: Expose the module in `lib.rs`**

Add to `crates/presentation/src/lib.rs`:

```rust
pub mod event_handlers;
```

- [ ] **Step 3: Run the test to verify it fails**

```bash
cargo test -p presentation event_handlers 2>&1 | tail -20
```

Expected: compile error or test failure because `handle` is `todo!()`.

- [ ] **Step 4: Implement `handle` in `PosterSyncHandler`**

Replace the `todo!()` body in `crates/presentation/src/event_handlers.rs`:

```rust
#[async_trait]
impl EventHandler for PosterSyncHandler {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError> {
        let (movie_id, external_metadata_id) = match event {
            DomainEvent::MovieDiscovered {
                movie_id,
                external_metadata_id,
            } => (movie_id.value(), external_metadata_id.value().to_owned()),
            _ => return Ok(()),
        };

        let mut last_err: Option<DomainError> = None;
        for attempt in 0..=self.max_retries {
            let cmd = SyncPosterCommand {
                movie_id,
                external_metadata_id: external_metadata_id.clone(),
            };
            match sync_poster::execute(&self.ctx, cmd).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    if attempt < self.max_retries {
                        let delay = Duration::from_secs(2u64.pow(attempt));
                        tracing::warn!(
                            attempt = attempt + 1,
                            max_attempts = self.max_retries + 1,
                            delay_secs = delay.as_secs(),
                            "poster sync failed, retrying: {e}"
                        );
                        tokio::time::sleep(delay).await;
                    }
                    last_err = Some(e);
                }
            }
        }

        let err = last_err.unwrap();
        tracing::error!(
            attempts = self.max_retries + 1,
            "poster sync failed after all attempts: {err}"
        );
        Err(err)
    }
}
```

- [ ] **Step 5: Run the test to verify it passes**

```bash
cargo test -p presentation event_handlers 2>&1 | tail -20
```

Expected: `test result: ok. 1 passed`

- [ ] **Step 6: Commit**

```bash
git add crates/presentation/src/event_handlers.rs crates/presentation/src/lib.rs
git commit -m "feat(presentation): implement PosterSyncHandler with retry"
```

---

## Task 4: Wire `PosterSyncHandler` in `main.rs`

**Files:**
- Modify: `crates/presentation/src/main.rs`

- [ ] **Step 1: Add the import**

In `crates/presentation/src/main.rs`, update the import block. The existing line is:

```rust
use event_publisher::{EventPublisherConfig, create_event_channel};
```

Add below it:

```rust
use presentation::event_handlers::PosterSyncHandler;
```

- [ ] **Step 2: Wire the handler**

In `wire_dependencies`, find the two existing lines:

```rust
let (event_publisher, event_worker) = create_event_channel(EventPublisherConfig::from_env());
tokio::spawn(event_worker.run());
```

Replace with:

```rust
let poster_handler = PosterSyncHandler::new(app_ctx.clone(), 3); // 3 retries = 4 total attempts
let (event_publisher, event_worker) = create_event_channel(
    EventPublisherConfig::from_env(),
    vec![Box::new(poster_handler)],
);
tokio::spawn(event_worker.run());
```

Note: `app_ctx.clone()` is cheap — all fields are `Arc<dyn Trait>`.

- [ ] **Step 3: Build the full workspace**

```bash
cargo build 2>&1 | tail -20
```

Expected: clean build with no errors.

- [ ] **Step 4: Run all tests**

```bash
cargo test 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/presentation/src/main.rs
git commit -m "feat(presentation): wire PosterSyncHandler into event worker"
```

---

## Verification

After all tasks complete, smoke-test end-to-end:

```bash
# Start the server
RUST_LOG=info cargo run -p presentation

# In another terminal: log a review for a movie not yet in the DB
# (requires valid JWT — use the existing login endpoint first)

# Watch the server logs for:
#   event: movie_discovered  movie_id=<uuid> external_id=tt...
#   poster sync attempt logs (or success with no retries needed)
```

To confirm the poster was stored, check the configured object store bucket/directory for a file named with the movie's UUID.
