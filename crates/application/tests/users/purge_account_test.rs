use std::sync::{Arc, Mutex};

use application::users::purge_account::AccountPurgeHandler;
use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::{DomainEvent, EventEnvelope, EventId},
    ports::{
        AccountPurger, EventConsumer, EventHandler, FederationDeletionNotifier, ObjectStorage,
        UserRepository,
    },
    testing::{InMemoryStore, NoopAckHandle, test_user},
    value_objects::UserId,
};
use futures::stream::{self, BoxStream};

type Log = Arc<Mutex<Vec<String>>>;

struct Notifier {
    log: Log,
    fail: bool,
}

#[async_trait]
impl FederationDeletionNotifier for Notifier {
    async fn notify_deleted(&self, _: &UserId) -> Result<(), DomainError> {
        self.log.lock().unwrap().push("notify".into());
        if self.fail {
            Err(DomainError::InfrastructureError("peer unreachable".into()))
        } else {
            Ok(())
        }
    }
}

struct Purger {
    log: Log,
    purged: Mutex<bool>,
    keys: Vec<String>,
}

#[async_trait]
impl AccountPurger for Purger {
    async fn is_purged(&self, _: &UserId) -> Result<bool, DomainError> {
        Ok(*self.purged.lock().unwrap())
    }
    async fn owned_image_keys(&self, _: &UserId) -> Result<Vec<String>, DomainError> {
        Ok(self.keys.clone())
    }
    async fn purge(&self, _: &UserId) -> Result<(), DomainError> {
        self.log.lock().unwrap().push("purge".into());
        *self.purged.lock().unwrap() = true;
        Ok(())
    }
    async fn forget_images(&self, _: &UserId) -> Result<(), DomainError> {
        self.log.lock().unwrap().push("forget".into());
        Ok(())
    }
    async fn purge_expired_tombstones(&self, _: chrono::NaiveDateTime) -> Result<u64, DomainError> {
        Ok(0)
    }
}

struct Storage {
    log: Log,
}

#[async_trait]
impl ObjectStorage for Storage {
    async fn store(&self, _: &str, _: &[u8]) -> Result<String, DomainError> {
        unimplemented!()
    }
    async fn get(&self, _: &str) -> Result<Vec<u8>, DomainError> {
        unimplemented!()
    }
    async fn get_stream(
        &self,
        _: &str,
    ) -> Result<BoxStream<'static, Result<bytes::Bytes, DomainError>>, DomainError> {
        unimplemented!()
    }
    async fn delete(&self, key: &str) -> Result<(), DomainError> {
        self.log.lock().unwrap().push(format!("delete {key}"));
        Ok(())
    }
    async fn exists(&self, _: &str) -> Result<bool, DomainError> {
        unimplemented!()
    }
}

struct Fixture {
    handler: Arc<AccountPurgeHandler>,
    purger: Arc<Purger>,
    log: Log,
    store: Arc<InMemoryStore>,
    user: UserId,
}

async fn fixture(notify_fails: bool, tombstoned: bool) -> Fixture {
    let store = InMemoryStore::new();
    let alice = test_user("alice");
    store.save(&alice).await.unwrap();
    if tombstoned {
        store.mark_deleted(alice.id(), &[]).await.unwrap();
    }
    let log: Log = Arc::default();
    let purger = Arc::new(Purger {
        log: log.clone(),
        purged: Mutex::new(false),
        keys: vec!["avatars/a.webp".into(), "banners/a.webp".into()],
    });
    let handler = Arc::new(AccountPurgeHandler::new(
        store.clone(),
        Arc::new(Notifier {
            log: log.clone(),
            fail: notify_fails,
        }),
        purger.clone(),
        Arc::new(Storage { log: log.clone() }),
    ));
    Fixture {
        handler,
        purger,
        log,
        store,
        user: alice.id().clone(),
    }
}

fn deleted(user: &UserId) -> DomainEvent {
    DomainEvent::UserDeleted {
        user_id: user.clone(),
    }
}

fn entries(log: &Log) -> Vec<String> {
    log.lock().unwrap().clone()
}

#[test]
fn the_handler_name_is_stable() {
    let f = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(fixture(false, true));
    assert_eq!(f.handler.name(), "account_purge");
}

#[tokio::test]
async fn notifies_first_then_purges_then_removes_images_then_forgets_them() {
    let f = fixture(false, true).await;

    f.handler.handle(&deleted(&f.user)).await.unwrap();

    assert_eq!(
        entries(&f.log),
        [
            "notify",
            "purge",
            "delete avatars/a.webp",
            "delete banners/a.webp",
            "forget"
        ]
    );
}

#[tokio::test]
async fn purge_continues_when_federation_notify_fails() {
    let f = fixture(true, true).await;

    f.handler.handle(&deleted(&f.user)).await.unwrap();

    let log = entries(&f.log);
    assert_eq!(log[0], "notify");
    assert!(log.contains(&"purge".to_string()), "{log:?}");
    assert!(log.contains(&"forget".to_string()), "{log:?}");
}

#[tokio::test]
async fn a_rerun_does_not_notify_again() {
    let f = fixture(false, true).await;
    f.handler.handle(&deleted(&f.user)).await.unwrap();

    f.handler.handle(&deleted(&f.user)).await.unwrap();

    let notices = entries(&f.log).iter().filter(|e| *e == "notify").count();
    assert_eq!(notices, 1);
}

#[tokio::test]
async fn does_nothing_when_the_account_is_not_tombstoned() {
    let live = fixture(false, false).await;
    live.handler.handle(&deleted(&live.user)).await.unwrap();
    assert!(entries(&live.log).is_empty(), "a live user is never purged");

    let unknown = fixture(false, true).await;
    unknown
        .handler
        .handle(&deleted(&UserId::generate()))
        .await
        .unwrap();
    assert!(entries(&unknown.log).is_empty());
}

#[tokio::test]
async fn ignores_other_events() {
    let f = fixture(false, true).await;
    f.handler
        .handle(&DomainEvent::UserUpdated {
            user_id: f.user.clone(),
        })
        .await
        .unwrap();
    assert!(entries(&f.log).is_empty());
}

struct TwiceConsumer {
    id: EventId,
    event: DomainEvent,
}

impl EventConsumer for TwiceConsumer {
    fn consume(&self) -> BoxStream<'_, Result<EventEnvelope, DomainError>> {
        let deliveries: Vec<Result<EventEnvelope, DomainError>> = (0..2)
            .map(|_| {
                Ok(EventEnvelope::new(
                    Some(self.id),
                    self.event.clone(),
                    Box::new(NoopAckHandle),
                ))
            })
            .collect();
        Box::pin(stream::iter(deliveries))
    }
}

#[tokio::test]
async fn a_redelivered_event_is_skipped_by_the_ledger() {
    let f = fixture(false, true).await;
    let consumer = TwiceConsumer {
        id: EventId::new(),
        event: deleted(&f.user),
    };

    application::worker::WorkerService::new(
        Arc::new(consumer),
        vec![f.handler.clone() as Arc<dyn EventHandler>],
        f.store.clone(),
    )
    .unwrap()
    .run(tokio::sync::watch::channel(false).1)
    .await;

    let purges = entries(&f.log).iter().filter(|e| *e == "purge").count();
    assert_eq!(purges, 1);
}

#[tokio::test]
async fn a_rerun_after_a_crash_following_the_commit_deletes_the_images() {
    let f = fixture(false, true).await;
    *f.purger.purged.lock().unwrap() = true;

    f.handler.handle(&deleted(&f.user)).await.unwrap();

    assert_eq!(
        entries(&f.log),
        [
            "purge",
            "delete avatars/a.webp",
            "delete banners/a.webp",
            "forget"
        ],
        "no second federation notice, images removed and forgotten"
    );
}
