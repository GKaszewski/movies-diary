use domain::{
    events::DomainEvent,
    ports::{MovieCommand, ReviewRepository, UserRepository},
    testing::{test_movie, test_review, test_user},
};
use sqlite::{SqliteMovieRepository, SqliteReviewRepository, SqliteUserRepository};
use sqlx::SqlitePool;

use super::outbox_support::{count, fail_event_inserts, migrated_pool};

fn logged(review: &domain::models::Review) -> DomainEvent {
    DomainEvent::ReviewLogged {
        review_id: review.id().clone(),
        movie_id: review.movie_id().clone(),
        user_id: review.user_id().clone(),
        rating: review.rating().clone(),
        watched_at: *review.watched_at(),
    }
}

#[tokio::test]
async fn saving_a_review_writes_the_review_and_its_event_in_one_commit() {
    let pool = migrated_pool().await;
    let movie = test_movie("Heat");
    let user = test_user("alice");
    SqliteUserRepository::new(pool.clone())
        .save(&user)
        .await
        .unwrap();
    SqliteMovieRepository::new(pool.clone())
        .upsert_movie(&movie, &[])
        .await
        .unwrap();
    let review = test_review(movie.id(), user.id(), 4);

    SqliteReviewRepository::new(pool.clone())
        .save_review(&review, &[logged(&review)])
        .await
        .unwrap();

    assert_eq!(count(&pool, "reviews").await, 1);
    assert_eq!(count(&pool, "event_queue").await, 1);
}

#[tokio::test]
async fn a_failed_review_save_leaves_no_event_row() {
    let pool = migrated_pool().await;
    let review = test_review(
        &domain::value_objects::MovieId::generate(),
        &domain::value_objects::UserId::generate(),
        4,
    );

    let result = SqliteReviewRepository::new(pool.clone())
        .save_review(&review, &[logged(&review)])
        .await;

    assert!(result.is_err(), "FK violation must fail the save");
    assert_eq!(count(&pool, "reviews").await, 0);
    assert_eq!(count(&pool, "event_queue").await, 0);
}

fn updated(review: &domain::models::Review) -> DomainEvent {
    DomainEvent::ReviewUpdated {
        review_id: review.id().clone(),
        movie_id: review.movie_id().clone(),
        user_id: review.user_id().clone(),
        rating: review.rating().clone(),
        watched_at: *review.watched_at(),
    }
}

async fn seeded() -> (SqlitePool, domain::models::Review) {
    let pool = migrated_pool().await;
    let movie = test_movie("Heat");
    let user = test_user("alice");
    SqliteUserRepository::new(pool.clone())
        .save(&user)
        .await
        .unwrap();
    SqliteMovieRepository::new(pool.clone())
        .upsert_movie(&movie, &[])
        .await
        .unwrap();
    let review = test_review(movie.id(), user.id(), 4);
    (pool, review)
}

#[tokio::test]
async fn a_failing_enqueue_after_the_review_insert_rolls_the_review_back() {
    let (pool, review) = seeded().await;
    fail_event_inserts(&pool).await;

    let result = SqliteReviewRepository::new(pool.clone())
        .save_review(&review, &[logged(&review)])
        .await;

    assert!(result.is_err());
    assert_eq!(count(&pool, "reviews").await, 0);
    assert_eq!(count(&pool, "event_queue").await, 0);
}

#[tokio::test]
async fn updating_a_review_writes_the_change_and_its_event_in_one_commit() {
    let (pool, review) = seeded().await;
    let repo = SqliteReviewRepository::new(pool.clone());
    repo.save_review(&review, &[]).await.unwrap();
    let event = updated(&review);

    repo.update_review(&review, &[event]).await.unwrap();

    assert_eq!(count(&pool, "event_queue").await, 1);
}

#[tokio::test]
async fn a_failing_enqueue_after_the_review_update_rolls_the_update_back() {
    let (pool, review) = seeded().await;
    let repo = SqliteReviewRepository::new(pool.clone());
    repo.save_review(&review, &[]).await.unwrap();
    fail_event_inserts(&pool).await;
    let edited = review.apply_edit(domain::models::ReviewEdit {
        rating: Some(domain::value_objects::Rating::new(1).unwrap()),
        ..Default::default()
    });
    let event = updated(&edited);

    let result = repo.update_review(&edited, &[event]).await;

    assert!(result.is_err());
    let persisted: i64 = sqlx::query_scalar("SELECT rating FROM reviews")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(persisted, 4, "rating update rolled back");
    assert_eq!(count(&pool, "event_queue").await, 0);
}

fn deleted(review: &domain::models::Review) -> DomainEvent {
    DomainEvent::ReviewDeleted {
        review_id: review.id().clone(),
        user_id: review.user_id().clone(),
    }
}

fn movie_deleted(review: &domain::models::Review) -> DomainEvent {
    DomainEvent::MovieDeleted {
        movie_id: review.movie_id().clone(),
        poster_path: None,
    }
}

#[tokio::test]
async fn deleting_the_last_review_removes_the_movie_and_enqueues_both_events_in_one_commit() {
    let (pool, review) = seeded().await;
    let repo = SqliteReviewRepository::new(pool.clone());
    repo.save_review(&review, &[]).await.unwrap();

    repo.delete_review_and_orphaned_movie(
        review.id(),
        review.movie_id(),
        &[deleted(&review)],
        &[movie_deleted(&review)],
    )
    .await
    .unwrap();

    assert_eq!(count(&pool, "reviews").await, 0);
    assert_eq!(count(&pool, "movies").await, 0);
    assert_eq!(count(&pool, "event_queue").await, 2);
}

#[tokio::test]
async fn deleting_a_review_while_another_remains_keeps_the_movie_and_enqueues_only_review_deleted()
{
    let (pool, review) = seeded().await;
    let repo = SqliteReviewRepository::new(pool.clone());
    repo.save_review(&review, &[]).await.unwrap();
    let other = test_review(review.movie_id(), review.user_id(), 3);
    repo.save_review(&other, &[]).await.unwrap();

    repo.delete_review_and_orphaned_movie(
        review.id(),
        review.movie_id(),
        &[deleted(&review)],
        &[movie_deleted(&review)],
    )
    .await
    .unwrap();

    assert_eq!(count(&pool, "reviews").await, 1);
    assert_eq!(count(&pool, "movies").await, 1);
    assert_eq!(count(&pool, "event_queue").await, 1);
}

#[tokio::test]
async fn a_failing_enqueue_rolls_back_the_review_and_movie_delete() {
    let (pool, review) = seeded().await;
    let repo = SqliteReviewRepository::new(pool.clone());
    repo.save_review(&review, &[]).await.unwrap();
    fail_event_inserts(&pool).await;

    let result = repo
        .delete_review_and_orphaned_movie(
            review.id(),
            review.movie_id(),
            &[deleted(&review)],
            &[movie_deleted(&review)],
        )
        .await;

    assert!(result.is_err());
    assert_eq!(
        count(&pool, "reviews").await,
        1,
        "review delete rolled back"
    );
    assert_eq!(count(&pool, "movies").await, 1, "movie delete rolled back");
    assert_eq!(count(&pool, "event_queue").await, 0);
}
