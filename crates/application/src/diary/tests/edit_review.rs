use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use domain::{
    models::Review,
    ports::ReviewRepository,
    testing::{InMemoryReviewRepository, NoopEventPublisher},
    value_objects::{Comment, MovieId, Rating, ReviewId, UserId, WatchMedium},
};

use crate::diary::{commands::EditReviewCommand, deps::EditReviewDeps, edit_review};

fn make_review(user_id: UserId) -> Review {
    Review::new(
        MovieId::generate(),
        user_id,
        Rating::new(3).unwrap(),
        Some(Comment::new("original comment".into()).unwrap()),
        Utc::now().naive_utc(),
        Some(WatchMedium::Streaming),
    )
    .unwrap()
}

async fn setup() -> (
    Arc<InMemoryReviewRepository>,
    Arc<NoopEventPublisher>,
    ReviewId,
    UserId,
) {
    let reviews = InMemoryReviewRepository::new();
    let events = NoopEventPublisher::new();
    let user_id = UserId::generate();
    let review = make_review(user_id.clone());
    let review_id = review.id().clone();
    reviews.save_review(&review).await.unwrap();
    (reviews, events, review_id, user_id)
}

fn deps(
    reviews: &Arc<InMemoryReviewRepository>,
    events: &Arc<NoopEventPublisher>,
) -> EditReviewDeps {
    EditReviewDeps {
        review: Arc::clone(reviews) as _,
        event_publisher: Arc::clone(events) as _,
    }
}

#[tokio::test]
async fn edit_own_review_updates_rating() {
    let (reviews, events, review_id, user_id) = setup().await;

    edit_review::execute(
        &deps(&reviews, &events),
        EditReviewCommand {
            review_id: review_id.value(),
            requesting_user_id: user_id.value(),
            rating: Some(5),
            comment: None,
            watched_at: None,
            watch_medium: None,
        },
    )
    .await
    .unwrap();

    let updated = reviews.get_review_by_id(&review_id).await.unwrap().unwrap();
    assert_eq!(updated.rating().value(), 5);
    assert_eq!(updated.comment().unwrap().value(), "original comment");
    assert_eq!(updated.watch_medium(), Some(&WatchMedium::Streaming));
}

#[tokio::test]
async fn edit_nonexistent_review_returns_not_found() {
    let reviews = InMemoryReviewRepository::new();
    let events = NoopEventPublisher::new();

    let err = edit_review::execute(
        &deps(&reviews, &events),
        EditReviewCommand {
            review_id: Uuid::new_v4(),
            requesting_user_id: Uuid::new_v4(),
            rating: Some(5),
            comment: None,
            watched_at: None,
            watch_medium: None,
        },
    )
    .await
    .unwrap_err();

    assert!(matches!(err, domain::errors::DomainError::NotFound(_)));
}

#[tokio::test]
async fn edit_other_users_review_returns_forbidden() {
    let (reviews, events, review_id, _user_id) = setup().await;

    let err = edit_review::execute(
        &deps(&reviews, &events),
        EditReviewCommand {
            review_id: review_id.value(),
            requesting_user_id: Uuid::new_v4(),
            rating: Some(5),
            comment: None,
            watched_at: None,
            watch_medium: None,
        },
    )
    .await
    .unwrap_err();

    assert!(matches!(err, domain::errors::DomainError::Forbidden(_)));
}

#[tokio::test]
async fn edit_publishes_review_updated_event() {
    let (reviews, events, review_id, user_id) = setup().await;

    edit_review::execute(
        &deps(&reviews, &events),
        EditReviewCommand {
            review_id: review_id.value(),
            requesting_user_id: user_id.value(),
            rating: Some(1),
            comment: None,
            watched_at: None,
            watch_medium: None,
        },
    )
    .await
    .unwrap();

    let published = events.published();
    assert_eq!(published.len(), 1);
    assert!(matches!(
        published[0],
        domain::events::DomainEvent::ReviewUpdated { .. }
    ));
}

#[tokio::test]
async fn edit_sets_watch_medium() {
    let (reviews, events, review_id, user_id) = setup().await;

    edit_review::execute(
        &deps(&reviews, &events),
        EditReviewCommand {
            review_id: review_id.value(),
            requesting_user_id: user_id.value(),
            rating: None,
            comment: None,
            watched_at: None,
            watch_medium: Some(Some(WatchMedium::Cinema)),
        },
    )
    .await
    .unwrap();

    let updated = reviews.get_review_by_id(&review_id).await.unwrap().unwrap();
    assert_eq!(updated.watch_medium(), Some(&WatchMedium::Cinema));
}
