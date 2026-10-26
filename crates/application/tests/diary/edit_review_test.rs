use crate::support::edit_review_command;
use std::sync::Arc;

use domain::{
    models::ReviewEdit,
    ports::ReviewRepository,
    testing::{InMemoryStore, test_review},
    value_objects::{Comment, MovieId, ReviewId, UserId, WatchMedium},
};

use application::diary::{commands::EditReviewCommand, deps::EditReviewDeps, edit_review};

async fn setup() -> (Arc<InMemoryStore>, ReviewId, UserId) {
    let reviews = InMemoryStore::new();
    let user_id = UserId::generate();
    let review = test_review(&MovieId::generate(), &user_id, 3).apply_edit(ReviewEdit {
        comment: Some(Some(Comment::new("original comment".into()).unwrap())),
        watch_medium: Some(Some(WatchMedium::Streaming)),
        ..Default::default()
    });
    let review_id = review.id().clone();
    reviews.save_review(&review, &[]).await.unwrap();
    (reviews, review_id, user_id)
}

fn deps(reviews: &Arc<InMemoryStore>) -> EditReviewDeps {
    EditReviewDeps {
        review: Arc::clone(reviews) as _,
    }
}

#[tokio::test]
async fn edit_own_review_updates_rating() {
    let (reviews, review_id, user_id) = setup().await;

    edit_review::execute(
        &deps(&reviews),
        EditReviewCommand {
            review_id: review_id.value(),
            requesting_user_id: user_id.value(),
            ..edit_review_command(5)
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
    let reviews = InMemoryStore::new();

    let err = edit_review::execute(&deps(&reviews), edit_review_command(5))
        .await
        .unwrap_err();

    assert!(matches!(err, domain::errors::DomainError::NotFound(_)));
}

#[tokio::test]
async fn edit_other_users_review_returns_forbidden() {
    let (reviews, review_id, _user_id) = setup().await;

    let err = edit_review::execute(
        &deps(&reviews),
        EditReviewCommand {
            review_id: review_id.value(),
            ..edit_review_command(5)
        },
    )
    .await
    .unwrap_err();

    assert!(matches!(err, domain::errors::DomainError::Forbidden(_)));
}

pub(crate) struct WriteRefusingReviews(pub(crate) Arc<InMemoryStore>);

#[async_trait::async_trait]
impl ReviewRepository for WriteRefusingReviews {
    async fn save_review(
        &self,
        _: &domain::models::Review,
        _: &[domain::events::DomainEvent],
    ) -> Result<(), domain::errors::DomainError> {
        Err(domain::errors::DomainError::InfrastructureError(
            "refused".into(),
        ))
    }
    async fn get_review_by_id(
        &self,
        id: &ReviewId,
    ) -> Result<Option<domain::models::Review>, domain::errors::DomainError> {
        self.0.get_review_by_id(id).await
    }
    async fn update_review(
        &self,
        _: &domain::models::Review,
        _: &[domain::events::DomainEvent],
    ) -> Result<(), domain::errors::DomainError> {
        Err(domain::errors::DomainError::InfrastructureError(
            "refused".into(),
        ))
    }
    async fn delete_review_and_orphaned_movie(
        &self,
        _: &ReviewId,
        _: &domain::value_objects::MovieId,
        _: &[domain::events::DomainEvent],
        _: &[domain::events::DomainEvent],
    ) -> Result<(), domain::errors::DomainError> {
        Err(domain::errors::DomainError::InfrastructureError(
            "refused".into(),
        ))
    }
    async fn get_all_reviews_for_user(
        &self,
        id: &UserId,
    ) -> Result<Vec<domain::models::Review>, domain::errors::DomainError> {
        self.0.get_all_reviews_for_user(id).await
    }
}

#[tokio::test]
async fn editing_a_review_saves_the_change_and_its_event_together() {
    let (reviews, review_id, user_id) = setup().await;
    assert!(reviews.recorded_events().is_empty());

    edit_review::execute(
        &deps(&reviews),
        EditReviewCommand {
            review_id: review_id.value(),
            requesting_user_id: user_id.value(),
            ..edit_review_command(1)
        },
    )
    .await
    .unwrap();

    let updated = reviews.get_review_by_id(&review_id).await.unwrap().unwrap();
    assert_eq!(updated.rating().value(), 1);
    let recorded = reviews.recorded_events();
    assert_eq!(recorded.len(), 1);
    assert!(matches!(
        recorded[0],
        domain::events::DomainEvent::ReviewUpdated { .. }
    ));
}

#[tokio::test]
async fn a_refused_edit_propagates_and_records_nothing() {
    let (reviews, review_id, user_id) = setup().await;
    let deps = EditReviewDeps {
        review: Arc::new(WriteRefusingReviews(Arc::clone(&reviews))),
    };

    let result = edit_review::execute(
        &deps,
        EditReviewCommand {
            review_id: review_id.value(),
            requesting_user_id: user_id.value(),
            ..edit_review_command(1)
        },
    )
    .await;

    assert!(result.is_err());
    let unchanged = reviews.get_review_by_id(&review_id).await.unwrap().unwrap();
    assert_eq!(unchanged.rating().value(), 3);
    assert!(reviews.recorded_events().is_empty());
}

#[tokio::test]
async fn edit_sets_watch_medium() {
    let (reviews, review_id, user_id) = setup().await;

    edit_review::execute(
        &deps(&reviews),
        EditReviewCommand {
            rating: None,
            review_id: review_id.value(),
            requesting_user_id: user_id.value(),
            watch_medium: Some(Some(WatchMedium::Cinema)),
            ..edit_review_command(1)
        },
    )
    .await
    .unwrap();

    let updated = reviews.get_review_by_id(&review_id).await.unwrap().unwrap();
    assert_eq!(updated.watch_medium(), Some(&WatchMedium::Cinema));
}

#[tokio::test]
async fn editing_a_review_fails_when_the_store_refuses_to_load_it() {
    let (_, review_id, user_id) = setup().await;
    let deps = EditReviewDeps {
        review: Arc::new(domain::testing::RefusingReviewRepository),
    };

    let result = edit_review::execute(
        &deps,
        EditReviewCommand {
            review_id: review_id.value(),
            requesting_user_id: user_id.value(),
            ..edit_review_command(5)
        },
    )
    .await;

    assert!(matches!(
        result,
        Err(domain::errors::DomainError::InfrastructureError(m)) if m == "refused"
    ));
}
