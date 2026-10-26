use domain::{
    models::collections::PageParams,
    models::{FeedSortBy, FollowingFilter},
    ports::DiaryQuery,
};
use sqlite::SqliteDiaryRepository;
use sqlx::SqlitePool;

async fn setup(pool: &SqlitePool) {
    sqlite::migrate(pool).await.unwrap();

    sqlx::query(
        "INSERT INTO users (id, email, username, password_hash, created_at) VALUES
         ('11111111-1111-1111-1111-111111111111', 'alice@example.com', 'alice', 'hash', '2024-01-01 00:00:00'),
         ('22222222-2222-2222-2222-222222222222', 'bob@example.com', 'bob', 'hash', '2024-01-01 00:00:00'),
         ('33333333-3333-3333-3333-333333333333', 'carol@remote.social', 'carol', 'hash', '2024-01-01 00:00:00')",
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO movies (id, title, release_year) VALUES
         ('aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'Inception', 2010),
         ('bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb', 'Interstellar', 2014),
         ('cccccccc-cccc-cccc-cccc-cccccccccccc', 'Dune', 2021)",
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO reviews (id, movie_id, user_id, rating, watched_at, created_at, remote_actor_url) VALUES
         ('a1a1a1a1-a1a1-a1a1-a1a1-a1a1a1a1a1a1', 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', '11111111-1111-1111-1111-111111111111', 5, '2024-01-01 00:00:00', '2024-01-01 00:00:00', NULL),
         ('b2b2b2b2-b2b2-b2b2-b2b2-b2b2b2b2b2b2', 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb', '22222222-2222-2222-2222-222222222222', 3, '2024-01-02 00:00:00', '2024-01-02 00:00:00', NULL),
         ('c3c3c3c3-c3c3-c3c3-c3c3-c3c3c3c3c3c3', 'cccccccc-cccc-cccc-cccc-cccccccccccc', '33333333-3333-3333-3333-333333333333', 4, '2024-01-03 00:00:00', '2024-01-03 00:00:00', 'https://remote.social/users/carol')",
    )
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn activity_feed_sorts_by_rating_descending() {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    setup(&pool).await;
    let repo = SqliteDiaryRepository::new(pool);

    let page = PageParams::new(Some(10), Some(0)).unwrap();
    let result = repo
        .query_activity_feed_filtered(&page, &FeedSortBy::Rating, None, None)
        .await
        .unwrap();

    let ratings: Vec<u8> = result
        .items
        .iter()
        .map(|e| e.review().rating().value())
        .collect();
    assert_eq!(ratings, vec![5, 4, 3]);
}

#[tokio::test]
async fn activity_feed_filters_by_title_search() {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    setup(&pool).await;
    let repo = SqliteDiaryRepository::new(pool);

    let page = PageParams::new(Some(10), Some(0)).unwrap();
    let result = repo
        .query_activity_feed_filtered(&page, &FeedSortBy::Date, Some("Dune"), None)
        .await
        .unwrap();

    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].movie().title().value(), "Dune");
}

#[tokio::test]
async fn activity_feed_restricts_to_followed_local_and_remote_authors() {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    setup(&pool).await;
    let repo = SqliteDiaryRepository::new(pool);

    let filter = FollowingFilter {
        local_user_ids: vec![
            uuid::Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
        ],
        remote_actor_urls: vec!["https://remote.social/users/carol".to_string()],
    };
    let page = PageParams::new(Some(10), Some(0)).unwrap();
    let result = repo
        .query_activity_feed_filtered(&page, &FeedSortBy::Date, None, Some(&filter))
        .await
        .unwrap();

    assert_eq!(result.items.len(), 2);
    let titles: Vec<String> = result
        .items
        .iter()
        .map(|e| e.movie().title().value().to_string())
        .collect();
    assert!(titles.contains(&"Inception".to_string()));
    assert!(titles.contains(&"Dune".to_string()));
}

#[tokio::test]
async fn movie_stats_for_local_review() {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    setup(&pool).await;
    let repo = SqliteDiaryRepository::new(pool);

    let movie_id = domain::value_objects::MovieId::from_uuid(
        uuid::Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
    );
    let stats = repo.get_movie_stats(&movie_id).await.unwrap();

    assert_eq!(stats.total_count, 1);
    assert_eq!(stats.federated_count, 0);
    assert!((stats.avg_rating.unwrap() - 5.0).abs() < 0.001);
    assert_eq!(stats.rating_histogram[4], 1);
    assert_eq!(stats.rating_histogram[0], 0);
}

#[tokio::test]
async fn movie_social_feed_returns_reviews_for_movie() {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    setup(&pool).await;
    let repo = SqliteDiaryRepository::new(pool);

    let movie_id = domain::value_objects::MovieId::from_uuid(
        uuid::Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
    );
    let page = PageParams::new(Some(10), Some(0)).unwrap();
    let result = repo.get_movie_social_feed(&movie_id, &page).await.unwrap();

    assert_eq!(result.total_count, 1);
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].movie().title().value(), "Inception");
    assert_eq!(result.items[0].review().rating().value(), 5);
    assert_eq!(result.items[0].user_display_name(), "alice");
    assert!(!result.items[0].review().is_remote());
}

#[tokio::test]
async fn movie_social_feed_marks_federated_review_as_remote() {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    setup(&pool).await;
    let repo = SqliteDiaryRepository::new(pool);

    let movie_id = domain::value_objects::MovieId::from_uuid(
        uuid::Uuid::parse_str("cccccccc-cccc-cccc-cccc-cccccccccccc").unwrap(),
    );
    let page = PageParams::new(Some(10), Some(0)).unwrap();
    let result = repo.get_movie_social_feed(&movie_id, &page).await.unwrap();

    assert_eq!(result.total_count, 1);
    assert_eq!(result.items.len(), 1);
    assert!(result.items[0].review().is_remote());
    assert_eq!(
        result.items[0].user_email(),
        "https://remote.social/users/carol"
    );
}

#[tokio::test]
async fn movie_social_feed_offset_past_end_returns_no_items() {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    setup(&pool).await;
    let repo = SqliteDiaryRepository::new(pool);

    let movie_id = domain::value_objects::MovieId::from_uuid(
        uuid::Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
    );
    let page = PageParams::new(Some(10), Some(5)).unwrap();
    let result = repo.get_movie_social_feed(&movie_id, &page).await.unwrap();

    assert_eq!(result.total_count, 1);
    assert_eq!(result.items.len(), 0);
}

#[tokio::test]
async fn movie_stats_counts_federated_review() {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    setup(&pool).await;
    let repo = SqliteDiaryRepository::new(pool);

    let movie_id = domain::value_objects::MovieId::from_uuid(
        uuid::Uuid::parse_str("cccccccc-cccc-cccc-cccc-cccccccccccc").unwrap(),
    );
    let stats = repo.get_movie_stats(&movie_id).await.unwrap();

    assert_eq!(stats.total_count, 1);
    assert_eq!(stats.federated_count, 1);
    assert_eq!(stats.rating_histogram[3], 1);
    assert_eq!(stats.rating_histogram[4], 0);
}

#[tokio::test]
async fn count_local_posts_excludes_remote_reviews() {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    setup(&pool).await;
    let repo = SqliteDiaryRepository::new(pool);

    let count = repo.count_local_posts().await.unwrap();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn feeds_hide_reviews_of_deleted_local_authors() {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    setup(&pool).await;
    sqlx::query("UPDATE users SET deleted_at = '2026-01-01T00:00:00Z' WHERE username = 'alice'")
        .execute(&pool)
        .await
        .unwrap();
    let repo = SqliteDiaryRepository::new(pool);

    let page = PageParams::new(Some(10), Some(0)).unwrap();
    let feed = repo
        .query_activity_feed_filtered(&page, &FeedSortBy::Date, None, None)
        .await
        .unwrap();
    assert_eq!(feed.total_count, 2);
    assert!(
        feed.items
            .iter()
            .all(|e| e.movie().title().value() != "Inception")
    );

    let movie = domain::value_objects::MovieId::from_uuid(
        uuid::Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
    );
    let social = repo.get_movie_social_feed(&movie, &page).await.unwrap();
    assert!(social.items.is_empty());
}
