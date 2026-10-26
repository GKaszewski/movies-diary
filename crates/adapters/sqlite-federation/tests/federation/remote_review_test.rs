use activitypub::{NewRemoteReview, RemoteReviewRepository};
use domain::{
    events::DomainEvent,
    models::{PersistedReview, Review, ReviewSource},
    value_objects::{Comment, ExternalMetadataId, MovieId, Rating, ReviewId, UserId},
};
use sqlite_federation::SqliteFederationRepository;
use sqlx::SqlitePool;

const ACTOR: &str = "https://remote.example/users/ann";
const AP_ID: &str = "https://remote.example/reviews/1";

async fn migrated_pool() -> SqlitePool {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlite::migrate(&pool).await.unwrap();
    pool
}

async fn count(pool: &SqlitePool, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(pool)
        .await
        .unwrap()
}

fn remote_review(movie_id: &MovieId) -> Review {
    let now = chrono::Utc::now().naive_utc();
    Review::from_persistence(PersistedReview {
        id: ReviewId::generate(),
        movie_id: movie_id.clone(),
        user_id: UserId::generate(),
        rating: Rating::new(4).unwrap(),
        comment: Some(Comment::new("good".into()).unwrap()),
        watched_at: now,
        created_at: now,
        source: ReviewSource::Remote {
            actor_url: ACTOR.into(),
        },
        watch_medium: None,
        spoilers: false,
    })
}

fn enrichment_requested(movie_id: &MovieId) -> DomainEvent {
    DomainEvent::MovieEnrichmentRequested {
        movie_id: movie_id.clone(),
        external_metadata_id: ExternalMetadataId::new("tmdb:603".into()).unwrap(),
    }
}

async fn save(pool: &SqlitePool, review: &Review, events: &[DomainEvent]) -> anyhow::Result<()> {
    SqliteFederationRepository::new(pool.clone())
        .save_remote_review(NewRemoteReview {
            review,
            ap_id: AP_ID,
            movie_title: "The Matrix",
            release_year: 1999,
            external_metadata_id: Some("tmdb:603"),
            poster_url: None,
            events,
        })
        .await
}

#[tokio::test]
async fn saving_a_remote_review_writes_it_and_its_event_in_one_commit() {
    let pool = migrated_pool().await;
    let movie_id = MovieId::generate();
    let review = remote_review(&movie_id);

    save(&pool, &review, &[enrichment_requested(&movie_id)])
        .await
        .unwrap();

    assert_eq!(count(&pool, "reviews").await, 1);
    assert_eq!(count(&pool, "movies").await, 1);
    assert_eq!(count(&pool, "event_queue").await, 1);
}

#[tokio::test]
async fn a_failing_enqueue_rolls_back_the_remote_review_and_its_movie() {
    let pool = migrated_pool().await;
    sqlx::query(
        "CREATE TRIGGER fail_event_insert BEFORE INSERT ON event_queue \
         BEGIN SELECT RAISE(ABORT, 'enqueue failed'); END",
    )
    .execute(&pool)
    .await
    .unwrap();
    let movie_id = MovieId::generate();
    let review = remote_review(&movie_id);

    let result = save(&pool, &review, &[enrichment_requested(&movie_id)]).await;

    assert!(result.is_err());
    assert_eq!(count(&pool, "reviews").await, 0);
    assert_eq!(count(&pool, "movies").await, 0);
}

#[tokio::test]
async fn a_remote_review_without_events_enqueues_nothing() {
    let pool = migrated_pool().await;
    let movie_id = MovieId::generate();

    save(&pool, &remote_review(&movie_id), &[]).await.unwrap();

    assert_eq!(count(&pool, "reviews").await, 1);
    assert_eq!(count(&pool, "event_queue").await, 0);
}
