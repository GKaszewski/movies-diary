use std::sync::Arc;

use chrono::Utc;

use domain::{
    models::{Movie, Review},
    ports::{MovieRepository, ReviewRepository},
    testing::{
        FakeDiaryRepository, InMemoryMovieRepository, InMemoryReviewRepository, NoopEventPublisher,
    },
    value_objects::{MovieId, MovieTitle, Rating, ReleaseYear, UserId},
};

use crate::{
    diary::commands::DeleteReviewCommand, diary::delete_review, test_helpers::TestContextBuilder,
};

fn make_movie() -> Movie {
    Movie::new(
        None,
        MovieTitle::new("Terminator".into()).unwrap(),
        ReleaseYear::new(1984).unwrap(),
        None,
        None,
    )
}

fn make_review(movie_id: MovieId, user_id: UserId) -> Review {
    Review::new(
        movie_id,
        user_id,
        Rating::new(4).unwrap(),
        None,
        Utc::now().naive_utc(),
    )
    .unwrap()
}

#[tokio::test]
async fn test_delete_review_removes_it() {
    let movies = InMemoryMovieRepository::new();
    let reviews = InMemoryReviewRepository::new();
    let diary = FakeDiaryRepository::new();
    let events = NoopEventPublisher::new();

    let movie = make_movie();
    let user_id = UserId::from_uuid(uuid::Uuid::new_v4());
    let review = make_review(movie.id().clone(), user_id.clone());

    movies.upsert_movie(&movie).await.unwrap();
    reviews.save_review(&review).await.unwrap();
    diary.seed_history(movie.clone(), vec![]);

    let ctx = TestContextBuilder::new()
        .with_movies(Arc::clone(&movies) as _)
        .with_reviews(Arc::clone(&reviews) as _)
        .with_diary(Arc::clone(&diary) as _)
        .with_event_publisher(Arc::clone(&events) as _)
        .build();

    delete_review::execute(
        &ctx,
        DeleteReviewCommand {
            review_id: review.id().value(),
            requesting_user_id: user_id.value(),
        },
    )
    .await
    .unwrap();

    assert_eq!(reviews.count(), 0, "review should be deleted");
    assert!(
        movies.get_movie_by_id(movie.id()).await.unwrap().is_none(),
        "movie should be deleted when no reviews remain"
    );
}

#[tokio::test]
async fn test_delete_review_wrong_user_is_unauthorized() {
    let reviews = InMemoryReviewRepository::new();

    let movie_id = MovieId::from_uuid(uuid::Uuid::new_v4());
    let owner_id = UserId::from_uuid(uuid::Uuid::new_v4());
    let other_id = uuid::Uuid::new_v4();
    let review = make_review(movie_id, owner_id);

    reviews.save_review(&review).await.unwrap();

    let ctx = TestContextBuilder::new()
        .with_reviews(Arc::clone(&reviews) as _)
        .build();

    let result = delete_review::execute(
        &ctx,
        DeleteReviewCommand {
            review_id: review.id().value(),
            requesting_user_id: other_id,
        },
    )
    .await;

    assert!(result.is_err(), "wrong user should not be able to delete");
    assert_eq!(reviews.count(), 1, "review should still exist");
}
