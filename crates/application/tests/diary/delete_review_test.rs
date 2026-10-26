use domain::testing::{test_movie_with_year, test_review};
use std::sync::Arc;

use domain::{
    ports::{MovieCommand, MovieQuery, ReviewRepository},
    testing::{FakeDiaryQuery, InMemoryStore},
    value_objects::{MovieId, UserId},
};

use application::{
    diary::commands::DeleteReviewCommand, diary::delete_review, diary::deps::DeleteReviewDeps,
};

#[tokio::test]
async fn deleting_a_review_removes_it() {
    let movies = InMemoryStore::new();
    let reviews = Arc::clone(&movies);
    let diary = FakeDiaryQuery::new();

    let movie = test_movie_with_year("Terminator", 1984);
    let user_id = UserId::from_uuid(uuid::Uuid::new_v4());
    let review = test_review(movie.id(), &user_id, 4);

    movies.upsert_movie(&movie, &[]).await.unwrap();
    reviews.save_review(&review, &[]).await.unwrap();
    diary.seed_history(movie.clone(), vec![]);

    let deps = DeleteReviewDeps {
        review: Arc::clone(&reviews) as _,
        diary: diary.clone() as _,
    };

    delete_review::execute(
        &deps,
        DeleteReviewCommand {
            review_id: review.id().value(),
            requesting_user_id: user_id.value(),
        },
    )
    .await
    .unwrap();

    assert_eq!(reviews.review_count(), 0, "review should be deleted");
    assert!(
        movies.get_movie_by_id(movie.id()).await.unwrap().is_none(),
        "movie should be deleted when no reviews remain"
    );
}

#[tokio::test]
async fn deleting_another_users_review_is_unauthorized() {
    let reviews = InMemoryStore::new();
    let diary = FakeDiaryQuery::new();

    let movie_id = MovieId::from_uuid(uuid::Uuid::new_v4());
    let owner_id = UserId::from_uuid(uuid::Uuid::new_v4());
    let other_id = uuid::Uuid::new_v4();
    let review = test_review(&movie_id, &owner_id, 4);

    reviews.save_review(&review, &[]).await.unwrap();

    let deps = DeleteReviewDeps {
        review: Arc::clone(&reviews) as _,
        diary: diary as _,
    };

    let result = delete_review::execute(
        &deps,
        DeleteReviewCommand {
            review_id: review.id().value(),
            requesting_user_id: other_id,
        },
    )
    .await;

    assert!(result.is_err(), "wrong user should not be able to delete");
    assert_eq!(reviews.review_count(), 1, "review should still exist");
}

#[tokio::test]
async fn deleting_a_review_removes_it_and_records_its_event_together() {
    let movies = InMemoryStore::new();
    let diary = FakeDiaryQuery::new();
    let movie = test_movie_with_year("Terminator", 1984);
    let user_id = UserId::from_uuid(uuid::Uuid::new_v4());
    let review = test_review(movie.id(), &user_id, 4);
    movies.upsert_movie(&movie, &[]).await.unwrap();
    movies.save_review(&review, &[]).await.unwrap();
    let other = test_review(movie.id(), &UserId::from_uuid(uuid::Uuid::new_v4()), 3);
    movies.save_review(&other, &[]).await.unwrap();
    diary.seed_history(movie.clone(), vec![review.clone()]);
    let deps = DeleteReviewDeps {
        review: Arc::clone(&movies) as _,
        diary: diary.clone() as _,
    };

    delete_review::execute(
        &deps,
        DeleteReviewCommand {
            review_id: review.id().value(),
            requesting_user_id: user_id.value(),
        },
    )
    .await
    .unwrap();

    assert_eq!(movies.review_count(), 1);
    let recorded = movies.recorded_events();
    assert_eq!(recorded.len(), 1);
    assert!(matches!(
        recorded[0],
        domain::events::DomainEvent::ReviewDeleted { .. }
    ));
}

#[tokio::test]
async fn a_refused_delete_propagates_and_records_nothing() {
    let movies = InMemoryStore::new();
    let diary = FakeDiaryQuery::new();
    let user_id = UserId::from_uuid(uuid::Uuid::new_v4());
    let review = test_review(&MovieId::generate(), &user_id, 4);
    movies.save_review(&review, &[]).await.unwrap();
    diary.seed_history(test_movie_with_year("Terminator", 1984), vec![]);
    let deps = DeleteReviewDeps {
        review: Arc::new(crate::edit_review_test::WriteRefusingReviews(Arc::clone(
            &movies,
        ))),
        diary: diary as _,
    };

    let result = delete_review::execute(
        &deps,
        DeleteReviewCommand {
            review_id: review.id().value(),
            requesting_user_id: user_id.value(),
        },
    )
    .await;

    assert!(result.is_err());
    assert_eq!(movies.review_count(), 1);
    assert!(movies.recorded_events().is_empty());
}

#[tokio::test]
async fn deleting_the_last_review_records_movie_deleted_with_the_movie_delete() {
    let movies = InMemoryStore::new();
    let diary = FakeDiaryQuery::new();
    let movie = test_movie_with_year("Terminator", 1984);
    let user_id = UserId::from_uuid(uuid::Uuid::new_v4());
    let review = test_review(movie.id(), &user_id, 4);
    movies.upsert_movie(&movie, &[]).await.unwrap();
    movies.save_review(&review, &[]).await.unwrap();
    diary.seed_history(movie.clone(), vec![]);
    let deps = DeleteReviewDeps {
        review: Arc::clone(&movies) as _,
        diary: diary as _,
    };

    delete_review::execute(
        &deps,
        DeleteReviewCommand {
            review_id: review.id().value(),
            requesting_user_id: user_id.value(),
        },
    )
    .await
    .unwrap();

    assert!(movies.get_movie_by_id(movie.id()).await.unwrap().is_none());
    let recorded = movies.recorded_events();
    assert!(matches!(
        recorded[0],
        domain::events::DomainEvent::ReviewDeleted { .. }
    ));
    assert!(matches!(
        &recorded[1],
        domain::events::DomainEvent::MovieDeleted { movie_id, .. } if movie_id == movie.id()
    ));
    assert_eq!(recorded.len(), 2);
}

#[tokio::test]
async fn deleting_a_review_while_viewings_remain_keeps_the_movie_and_records_no_movie_deleted() {
    let movies = InMemoryStore::new();
    let diary = FakeDiaryQuery::new();
    let movie = test_movie_with_year("Terminator", 1984);
    let user_id = UserId::from_uuid(uuid::Uuid::new_v4());
    let review = test_review(movie.id(), &user_id, 4);
    let other = test_review(movie.id(), &UserId::from_uuid(uuid::Uuid::new_v4()), 3);
    movies.upsert_movie(&movie, &[]).await.unwrap();
    movies.save_review(&review, &[]).await.unwrap();
    movies.save_review(&other, &[]).await.unwrap();
    diary.seed_history(movie.clone(), vec![other.clone()]);
    let deps = DeleteReviewDeps {
        review: Arc::clone(&movies) as _,
        diary: diary as _,
    };

    delete_review::execute(
        &deps,
        DeleteReviewCommand {
            review_id: review.id().value(),
            requesting_user_id: user_id.value(),
        },
    )
    .await
    .unwrap();

    assert!(movies.get_movie_by_id(movie.id()).await.unwrap().is_some());
    assert!(
        !movies
            .recorded_events()
            .iter()
            .any(|e| matches!(e, domain::events::DomainEvent::MovieDeleted { .. }))
    );
}

#[tokio::test]
async fn a_refused_review_and_movie_delete_leaves_review_and_movie_and_records_nothing() {
    let movies = InMemoryStore::new();
    let diary = FakeDiaryQuery::new();
    let movie = test_movie_with_year("Terminator", 1984);
    let user_id = UserId::from_uuid(uuid::Uuid::new_v4());
    let review = test_review(movie.id(), &user_id, 4);
    movies.upsert_movie(&movie, &[]).await.unwrap();
    movies.save_review(&review, &[]).await.unwrap();
    diary.seed_history(movie.clone(), vec![]);
    let deps = DeleteReviewDeps {
        review: Arc::new(crate::edit_review_test::WriteRefusingReviews(Arc::clone(
            &movies,
        ))),
        diary: diary as _,
    };

    let result = delete_review::execute(
        &deps,
        DeleteReviewCommand {
            review_id: review.id().value(),
            requesting_user_id: user_id.value(),
        },
    )
    .await;

    assert!(result.is_err());
    assert_eq!(movies.review_count(), 1);
    assert!(movies.get_movie_by_id(movie.id()).await.unwrap().is_some());
    assert!(movies.recorded_events().is_empty());
}
