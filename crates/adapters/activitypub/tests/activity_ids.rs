use std::sync::Arc;
use std::time::Duration;

use activitypub::ActivityPubEventHandler;
use async_trait::async_trait;
use chrono::NaiveDate;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{DiaryEntry, WatchlistWithMovie},
    ports::{
        EventHandler, FederationDeletionNotifier, LocalApContentQuery, MovieCommand, UserRepository,
    },
    testing::{
        InMemoryStore, PanicGoalQuery, PanicReviewRepository, PanicStatsRepository, test_movie,
    },
    value_objects::{InstanceIdentity, MovieId, UserId},
};
use k_ap::{
    ActivityPubService, ApUser, FederationEvent,
    testing::{MockActorRepoBuilder, MockFollowRepoBuilder, MockUserRepoBuilder},
};
use tokio::sync::mpsc;

struct NoContent;

#[async_trait]
impl LocalApContentQuery for NoContent {
    async fn get_local_watchlist_for_user(
        &self,
        _: &UserId,
    ) -> Result<Vec<WatchlistWithMovie>, DomainError> {
        Ok(vec![])
    }
    async fn get_local_reviews_for_movie(
        &self,
        _: &MovieId,
    ) -> Result<Vec<DiaryEntry>, DomainError> {
        Ok(vec![])
    }
    async fn get_local_reviews_page(
        &self,
        _: &UserId,
        _: Option<chrono::NaiveDateTime>,
        _: usize,
    ) -> Result<Vec<DiaryEntry>, DomainError> {
        Ok(vec![])
    }
}

struct Capture(mpsc::UnboundedSender<serde_json::Value>);

#[async_trait]
impl k_ap::EventPublisher for Capture {
    async fn publish(&self, event: FederationEvent) -> anyhow::Result<()> {
        if let FederationEvent::DeliveryRequested { activity, .. } = event {
            let _ = self.0.send(activity);
        }
        Ok(())
    }
}

struct Harness {
    handler: ActivityPubEventHandler,
    delivered: mpsc::UnboundedReceiver<serde_json::Value>,
    user: UserId,
    movie_id: MovieId,
    store: Arc<InMemoryStore>,
    service: Arc<ActivityPubService>,
}

async fn harness() -> Harness {
    let user = UserId::from_uuid(uuid::Uuid::new_v4());
    let uid = user.value();
    let (tx, rx) = mpsc::unbounded_channel();
    let service = ActivityPubService::builder("https://diary.example")
        .activity_repo(k_ap::testing::MockActivityRepoBuilder::new().build())
        .follow_repo(
            MockFollowRepoBuilder::new()
                .on_get_accepted_follower_inboxes(|_| {
                    Ok(vec!["https://remote.example/inbox".to_string()])
                })
                .build(),
        )
        .actor_repo(MockActorRepoBuilder::new().build())
        .blocklist_repo(k_ap::testing::MockBlocklistRepoBuilder::new().build())
        .user_repo(
            MockUserRepoBuilder::new()
                .on_find_by_id(move |id| {
                    Ok((id == uid).then(|| ApUser {
                        id,
                        username: "alice".into(),
                        display_name: None,
                        bio: None,
                        avatar_url: None,
                        banner_url: None,
                        also_known_as: vec![],
                        profile_url: None,
                        attachment: vec![],
                        manually_approves_followers: false,
                        discoverable: true,
                        actor_type: Default::default(),
                        featured_url: None,
                    }))
                })
                .build(),
        )
        .content_reader(k_ap::testing::MockContentReaderBuilder::new().build())
        .object_handler(k_ap::testing::MockObjectHandlerBuilder::new().build())
        .event_publisher(Arc::new(Capture(tx)))
        .build()
        .await
        .expect("service builds");

    let store = InMemoryStore::new();
    let movie = test_movie("Heat");
    store.upsert_movie(&movie, &[]).await.unwrap();
    let service = Arc::new(service);
    store
        .save(&domain::testing::test_user_with_role(
            user.clone(),
            domain::models::UserRole::Standard,
        ))
        .await
        .unwrap();
    let handler = ActivityPubEventHandler::new(
        Arc::clone(&service),
        Arc::new(NoContent),
        Arc::new(PanicReviewRepository),
        store.clone(),
        Arc::new(PanicGoalQuery),
        Arc::new(PanicStatsRepository),
        store.clone(),
        store.clone(),
        InstanceIdentity::new("https://diary.example"),
    );
    Harness {
        handler,
        delivered: rx,
        user,
        movie_id: movie.id().clone(),
        store,
        service,
    }
}

impl Harness {
    async fn next_activity_id(&mut self) -> String {
        let activity = tokio::time::timeout(Duration::from_secs(5), self.delivered.recv())
            .await
            .expect("an activity was delivered")
            .expect("channel open");
        activity["id"]
            .as_str()
            .expect("activity has an id")
            .to_string()
    }
}

#[tokio::test]
async fn activitypub_is_idempotent_when_run_twice() {
    let mut h = harness().await;
    let event = DomainEvent::WatchlistEntryAdded {
        user_id: h.user.clone(),
        movie_id: h.movie_id.clone(),
        movie_title: "Heat".into(),
        release_year: 1995,
        external_metadata_id: None,
        added_at: NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap(),
    };

    h.handler.handle(&event).await.unwrap();
    let first = h.next_activity_id().await;
    h.handler.handle(&event).await.unwrap();
    let second = h.next_activity_id().await;

    assert!(first.starts_with("https://diary.example/activities/create/"));
    assert_eq!(first, second, "a redelivered Create keeps its activity id");
}

#[tokio::test]
async fn redelivered_delete_gets_a_fresh_activity_id_known_k_ap_limitation() {
    let mut h = harness().await;
    let event = DomainEvent::WatchlistEntryRemoved {
        user_id: h.user.clone(),
        movie_id: h.movie_id.clone(),
    };

    h.handler.handle(&event).await.unwrap();
    let first = h.next_activity_id().await;
    h.handler.handle(&event).await.unwrap();
    let second = h.next_activity_id().await;

    assert_ne!(first, second);
}

#[tokio::test]
async fn a_deleted_account_sends_nothing_but_its_delete() {
    let mut h = harness().await;
    h.store.mark_deleted(&h.user, &[]).await.unwrap();
    let logged = DomainEvent::ReviewLogged {
        review_id: domain::value_objects::ReviewId::generate(),
        user_id: h.user.clone(),
        movie_id: h.movie_id.clone(),
        rating: domain::value_objects::Rating::new(4).unwrap(),
        watched_at: NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap(),
    };
    let added = DomainEvent::WatchlistEntryAdded {
        user_id: h.user.clone(),
        movie_id: h.movie_id.clone(),
        movie_title: "Heat".into(),
        release_year: 1995,
        external_metadata_id: None,
        added_at: NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap(),
    };
    let queued = DomainEvent::FederationDeliveryRequested {
        inbox_url: "https://remote.example/inbox".into(),
        activity_json: serde_json::json!({"type": "Create"}),
        signing_actor_id: h.user.value(),
    };

    for event in [&logged, &added, &queued] {
        h.handler.handle(event).await.unwrap();
    }
    assert!(
        tokio::time::timeout(Duration::from_millis(300), h.delivered.recv())
            .await
            .is_err(),
        "nothing is delivered for a deleted account"
    );

    activitypub::ApDeletionNotifier::new(
        Arc::clone(&h.service),
        InstanceIdentity::new("https://diary.example"),
    )
    .notify_deleted(&h.user)
    .await
    .unwrap();
    let delete = tokio::time::timeout(Duration::from_secs(5), h.delivered.recv())
        .await
        .expect("the Delete is delivered")
        .unwrap();
    assert_eq!(delete["type"], "Delete");
}
