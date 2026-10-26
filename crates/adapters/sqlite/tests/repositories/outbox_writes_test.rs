use domain::{
    events::DomainEvent,
    models::{ProfileField, UserProfile, WatchlistEntry},
    ports::{
        GoalCommand, MovieCommand, UserProfileFieldsRepository, UserRepository, WatchEventCommand,
        WatchlistRepository, WrapUpRepository,
    },
    testing::{test_goal, test_movie, test_user},
    value_objects::UserId,
};
use sqlite::{
    SqliteGoalRepository, SqliteMovieRepository, SqliteProfileFieldsRepository,
    SqliteUserRepository, SqliteWatchlistRepository,
};
use sqlx::SqlitePool;

use super::outbox_support::{count, fail_event_inserts, migrated_pool};

async fn pool_with_user() -> (SqlitePool, domain::models::User) {
    let pool = migrated_pool().await;
    let user = test_user("alice");
    SqliteUserRepository::new(pool.clone())
        .save(&user)
        .await
        .unwrap();
    (pool, user)
}

fn goal_created(goal: &domain::models::Goal) -> DomainEvent {
    DomainEvent::GoalCreated {
        goal_id: goal.id().clone(),
        user_id: goal.user_id().clone(),
        year: goal.year(),
        target_count: goal.target_count(),
    }
}

fn goal_updated(goal: &domain::models::Goal) -> DomainEvent {
    DomainEvent::GoalUpdated {
        goal_id: goal.id().clone(),
        user_id: goal.user_id().clone(),
        year: goal.year(),
        target_count: goal.target_count(),
    }
}

fn goal_deleted(goal: &domain::models::Goal) -> DomainEvent {
    DomainEvent::GoalDeleted {
        goal_id: goal.id().clone(),
        user_id: goal.user_id().clone(),
        year: goal.year(),
    }
}

#[tokio::test]
async fn saving_a_goal_writes_the_goal_and_its_event_in_one_commit() {
    let (pool, user) = pool_with_user().await;
    let goal = test_goal(user.id(), 2025, 50);

    SqliteGoalRepository::new(pool.clone())
        .save(&goal, &[goal_created(&goal)])
        .await
        .unwrap();

    assert_eq!(count(&pool, "goals").await, 1);
    assert_eq!(count(&pool, "event_queue").await, 1);
}

#[tokio::test]
async fn a_failing_enqueue_rolls_back_the_goal_save() {
    let (pool, user) = pool_with_user().await;
    let goal = test_goal(user.id(), 2025, 50);
    fail_event_inserts(&pool).await;

    let result = SqliteGoalRepository::new(pool.clone())
        .save(&goal, &[goal_created(&goal)])
        .await;

    assert!(result.is_err());
    assert_eq!(count(&pool, "goals").await, 0);
}

#[tokio::test]
async fn updating_a_goal_writes_the_change_and_its_event_in_one_commit() {
    let (pool, user) = pool_with_user().await;
    let repo = SqliteGoalRepository::new(pool.clone());
    let mut goal = test_goal(user.id(), 2025, 50);
    repo.save(&goal, &[]).await.unwrap();
    goal.update_target(80).unwrap();

    repo.update(&goal, &[goal_updated(&goal)]).await.unwrap();

    assert_eq!(count(&pool, "event_queue").await, 1);
}

#[tokio::test]
async fn a_failing_enqueue_rolls_back_the_goal_update() {
    let (pool, user) = pool_with_user().await;
    let repo = SqliteGoalRepository::new(pool.clone());
    let mut goal = test_goal(user.id(), 2025, 50);
    repo.save(&goal, &[]).await.unwrap();
    goal.update_target(80).unwrap();
    fail_event_inserts(&pool).await;

    let result = repo.update(&goal, &[goal_updated(&goal)]).await;

    assert!(result.is_err());
    let target: i64 = sqlx::query_scalar("SELECT target_count FROM goals")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(target, 50, "update rolled back");
}

#[tokio::test]
async fn updating_a_missing_goal_enqueues_no_event() {
    let (pool, user) = pool_with_user().await;
    let goal = test_goal(user.id(), 2025, 50);

    let result = SqliteGoalRepository::new(pool.clone())
        .update(&goal, &[goal_updated(&goal)])
        .await;

    assert!(result.is_err());
    assert_eq!(count(&pool, "event_queue").await, 0);
}

#[tokio::test]
async fn deleting_a_goal_writes_the_removal_and_its_event_in_one_commit() {
    let (pool, user) = pool_with_user().await;
    let repo = SqliteGoalRepository::new(pool.clone());
    let goal = test_goal(user.id(), 2025, 50);
    repo.save(&goal, &[]).await.unwrap();

    repo.delete(goal.id(), user.id(), &[goal_deleted(&goal)])
        .await
        .unwrap();

    assert_eq!(count(&pool, "goals").await, 0);
    assert_eq!(count(&pool, "event_queue").await, 1);
}

#[tokio::test]
async fn a_failing_enqueue_rolls_back_the_goal_delete() {
    let (pool, user) = pool_with_user().await;
    let repo = SqliteGoalRepository::new(pool.clone());
    let goal = test_goal(user.id(), 2025, 50);
    repo.save(&goal, &[]).await.unwrap();
    fail_event_inserts(&pool).await;

    let result = repo
        .delete(goal.id(), user.id(), &[goal_deleted(&goal)])
        .await;

    assert!(result.is_err());
    assert_eq!(count(&pool, "goals").await, 1, "delete rolled back");
}

async fn watchlist_fixture() -> (SqlitePool, UserId, domain::models::Movie) {
    let (pool, user) = pool_with_user().await;
    let movie = test_movie("Heat");
    SqliteMovieRepository::new(pool.clone())
        .upsert_movie(&movie, &[])
        .await
        .unwrap();
    (pool, user.id().clone(), movie)
}

fn watchlist_added(entry: &WatchlistEntry, movie: &domain::models::Movie) -> DomainEvent {
    DomainEvent::WatchlistEntryAdded {
        user_id: entry.user_id.clone(),
        movie_id: movie.id().clone(),
        movie_title: movie.title().value().to_string(),
        release_year: movie.release_year().value(),
        external_metadata_id: None,
        added_at: entry.added_at,
    }
}

fn watchlist_removed(user_id: &UserId, movie: &domain::models::Movie) -> DomainEvent {
    DomainEvent::WatchlistEntryRemoved {
        user_id: user_id.clone(),
        movie_id: movie.id().clone(),
    }
}

#[tokio::test]
async fn adding_a_watchlist_entry_writes_the_entry_and_its_event_in_one_commit() {
    let (pool, user_id, movie) = watchlist_fixture().await;
    let entry = WatchlistEntry::new(user_id, movie.id().clone());

    SqliteWatchlistRepository::new(pool.clone())
        .add(&entry, &[watchlist_added(&entry, &movie)])
        .await
        .unwrap();

    assert_eq!(count(&pool, "watchlist_entries").await, 1);
    assert_eq!(count(&pool, "event_queue").await, 1);
}

#[tokio::test]
async fn re_adding_a_watchlist_entry_enqueues_no_second_event() {
    let (pool, user_id, movie) = watchlist_fixture().await;
    let repo = SqliteWatchlistRepository::new(pool.clone());
    let first = WatchlistEntry::new(user_id.clone(), movie.id().clone());
    repo.add(&first, &[watchlist_added(&first, &movie)])
        .await
        .unwrap();
    let again = WatchlistEntry::new(user_id, movie.id().clone());

    repo.add(&again, &[watchlist_added(&again, &movie)])
        .await
        .unwrap();

    assert_eq!(count(&pool, "watchlist_entries").await, 1);
    assert_eq!(count(&pool, "event_queue").await, 1, "no phantom event");
}

#[tokio::test]
async fn a_failing_enqueue_rolls_back_the_watchlist_add() {
    let (pool, user_id, movie) = watchlist_fixture().await;
    let entry = WatchlistEntry::new(user_id, movie.id().clone());
    fail_event_inserts(&pool).await;

    let result = SqliteWatchlistRepository::new(pool.clone())
        .add(&entry, &[watchlist_added(&entry, &movie)])
        .await;

    assert!(result.is_err());
    assert_eq!(count(&pool, "watchlist_entries").await, 0);
}

#[tokio::test]
async fn removing_a_watchlist_entry_writes_the_removal_and_its_event_in_one_commit() {
    let (pool, user_id, movie) = watchlist_fixture().await;
    let repo = SqliteWatchlistRepository::new(pool.clone());
    repo.add(
        &WatchlistEntry::new(user_id.clone(), movie.id().clone()),
        &[],
    )
    .await
    .unwrap();

    repo.remove(&user_id, movie.id(), &[watchlist_removed(&user_id, &movie)])
        .await
        .unwrap();

    assert_eq!(count(&pool, "watchlist_entries").await, 0);
    assert_eq!(count(&pool, "event_queue").await, 1);
}

#[tokio::test]
async fn a_failing_enqueue_rolls_back_the_watchlist_remove() {
    let (pool, user_id, movie) = watchlist_fixture().await;
    let repo = SqliteWatchlistRepository::new(pool.clone());
    repo.add(
        &WatchlistEntry::new(user_id.clone(), movie.id().clone()),
        &[],
    )
    .await
    .unwrap();
    fail_event_inserts(&pool).await;

    let result = repo
        .remove(&user_id, movie.id(), &[watchlist_removed(&user_id, &movie)])
        .await;

    assert!(result.is_err());
    assert_eq!(
        count(&pool, "watchlist_entries").await,
        1,
        "remove rolled back"
    );
}

#[tokio::test]
async fn removing_an_absent_watchlist_entry_enqueues_no_event() {
    let (pool, user_id, movie) = watchlist_fixture().await;

    let result = SqliteWatchlistRepository::new(pool.clone())
        .remove(&user_id, movie.id(), &[watchlist_removed(&user_id, &movie)])
        .await;

    assert!(result.is_err());
    assert_eq!(count(&pool, "event_queue").await, 0);
}

#[tokio::test]
async fn remove_if_present_enqueues_its_event_only_when_a_row_was_removed() {
    let (pool, user_id, movie) = watchlist_fixture().await;
    let repo = SqliteWatchlistRepository::new(pool.clone());
    let event = watchlist_removed(&user_id, &movie);

    let absent = repo
        .remove_if_present(&user_id, movie.id(), std::slice::from_ref(&event))
        .await
        .unwrap();
    assert!(!absent);
    assert_eq!(count(&pool, "event_queue").await, 0);

    repo.add(
        &WatchlistEntry::new(user_id.clone(), movie.id().clone()),
        &[],
    )
    .await
    .unwrap();
    let present = repo
        .remove_if_present(&user_id, movie.id(), std::slice::from_ref(&event))
        .await
        .unwrap();

    assert!(present);
    assert_eq!(count(&pool, "watchlist_entries").await, 0);
    assert_eq!(count(&pool, "event_queue").await, 1);
}

#[tokio::test]
async fn a_failing_enqueue_rolls_back_remove_if_present() {
    let (pool, user_id, movie) = watchlist_fixture().await;
    let repo = SqliteWatchlistRepository::new(pool.clone());
    repo.add(
        &WatchlistEntry::new(user_id.clone(), movie.id().clone()),
        &[],
    )
    .await
    .unwrap();
    fail_event_inserts(&pool).await;

    let result = repo
        .remove_if_present(&user_id, movie.id(), &[watchlist_removed(&user_id, &movie)])
        .await;

    assert!(result.is_err());
    assert_eq!(
        count(&pool, "watchlist_entries").await,
        1,
        "remove rolled back"
    );
}

#[tokio::test]
async fn updating_a_profile_writes_the_change_and_its_event_in_one_commit() {
    let (pool, user) = pool_with_user().await;
    let repo = SqliteUserRepository::new(pool.clone());
    let profile = UserProfile {
        bio: Some("hello".into()),
        ..Default::default()
    };

    repo.update_profile(
        user.id(),
        &profile,
        &[DomainEvent::UserUpdated {
            user_id: user.id().clone(),
        }],
    )
    .await
    .unwrap();

    let found = repo.find_by_id(user.id()).await.unwrap().unwrap();
    assert_eq!(found.bio(), Some("hello"));
    assert_eq!(count(&pool, "event_queue").await, 1);
}

#[tokio::test]
async fn a_failing_enqueue_rolls_back_the_profile_update() {
    let (pool, user) = pool_with_user().await;
    let repo = SqliteUserRepository::new(pool.clone());
    fail_event_inserts(&pool).await;

    let result = repo
        .update_profile(
            user.id(),
            &UserProfile {
                bio: Some("hello".into()),
                ..Default::default()
            },
            &[DomainEvent::UserUpdated {
                user_id: user.id().clone(),
            }],
        )
        .await;

    assert!(result.is_err());
    let found = repo.find_by_id(user.id()).await.unwrap().unwrap();
    assert_eq!(found.bio(), None, "profile update rolled back");
}

fn fields() -> Vec<ProfileField> {
    vec![ProfileField {
        name: "Site".into(),
        value: "https://example.com".into(),
    }]
}

#[tokio::test]
async fn setting_profile_fields_writes_them_and_the_event_in_one_commit() {
    let (pool, user) = pool_with_user().await;

    SqliteProfileFieldsRepository::new(pool.clone())
        .set_fields(
            user.id(),
            fields(),
            &[DomainEvent::UserUpdated {
                user_id: user.id().clone(),
            }],
        )
        .await
        .unwrap();

    assert_eq!(count(&pool, "user_profile_fields").await, 1);
    assert_eq!(count(&pool, "event_queue").await, 1);
}

#[tokio::test]
async fn a_failing_enqueue_rolls_back_the_profile_fields_replacement() {
    let (pool, user) = pool_with_user().await;
    let repo = SqliteProfileFieldsRepository::new(pool.clone());
    repo.set_fields(user.id(), fields(), &[]).await.unwrap();
    fail_event_inserts(&pool).await;

    let result = repo
        .set_fields(
            user.id(),
            vec![],
            &[DomainEvent::UserUpdated {
                user_id: user.id().clone(),
            }],
        )
        .await;

    assert!(result.is_err());
    assert_eq!(
        repo.get_fields(user.id()).await.unwrap().len(),
        1,
        "delete of the old fields rolled back"
    );
}

fn movie_discovered(movie: &domain::models::Movie) -> DomainEvent {
    DomainEvent::MovieDiscovered {
        movie_id: movie.id().clone(),
        external_metadata_id: domain::value_objects::ExternalMetadataId::new("tmdb:1".into())
            .unwrap(),
    }
}

fn movie_deleted(movie: &domain::models::Movie) -> DomainEvent {
    DomainEvent::MovieDeleted {
        movie_id: movie.id().clone(),
        poster_path: None,
    }
}

#[tokio::test]
async fn upserting_a_movie_writes_the_movie_and_its_event_in_one_commit() {
    let pool = migrated_pool().await;
    let movie = test_movie("Heat");

    SqliteMovieRepository::new(pool.clone())
        .upsert_movie(&movie, &[movie_discovered(&movie)])
        .await
        .unwrap();

    assert_eq!(count(&pool, "movies").await, 1);
    assert_eq!(count(&pool, "event_queue").await, 1);
}

#[tokio::test]
async fn a_failing_enqueue_rolls_back_the_movie_upsert_of_a_new_movie() {
    let pool = migrated_pool().await;
    let movie = test_movie("Heat");
    fail_event_inserts(&pool).await;

    let result = SqliteMovieRepository::new(pool.clone())
        .upsert_movie(&movie, &[movie_discovered(&movie)])
        .await;

    assert!(result.is_err());
    assert_eq!(count(&pool, "movies").await, 0);
}

#[tokio::test]
async fn a_failing_enqueue_rolls_back_a_movie_update() {
    let pool = migrated_pool().await;
    let repo = SqliteMovieRepository::new(pool.clone());
    let mut movie = test_movie("Heat");
    repo.upsert_movie(&movie, &[]).await.unwrap();
    movie.update_poster(domain::value_objects::PosterPath::new("posters/new.jpg".into()).unwrap());
    fail_event_inserts(&pool).await;

    let result = repo.upsert_movie(&movie, &[movie_discovered(&movie)]).await;

    assert!(result.is_err());
    let poster: Option<String> = sqlx::query_scalar("SELECT poster_path FROM movies")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(poster, None, "poster update rolled back");
}

#[tokio::test]
async fn deleting_a_movie_writes_the_removal_and_its_event_in_one_commit() {
    let pool = migrated_pool().await;
    let repo = SqliteMovieRepository::new(pool.clone());
    let movie = test_movie("Heat");
    repo.upsert_movie(&movie, &[]).await.unwrap();

    repo.delete_movie(movie.id(), &[movie_deleted(&movie)])
        .await
        .unwrap();

    assert_eq!(count(&pool, "movies").await, 0);
    assert_eq!(count(&pool, "event_queue").await, 1);
}

#[tokio::test]
async fn a_failing_enqueue_rolls_back_the_movie_delete() {
    let pool = migrated_pool().await;
    let repo = SqliteMovieRepository::new(pool.clone());
    let movie = test_movie("Heat");
    repo.upsert_movie(&movie, &[]).await.unwrap();
    fail_event_inserts(&pool).await;

    let result = repo
        .delete_movie(movie.id(), &[movie_deleted(&movie)])
        .await;

    assert!(result.is_err());
    assert_eq!(count(&pool, "movies").await, 1, "delete rolled back");
}

#[tokio::test]
async fn deleting_an_absent_movie_enqueues_no_event() {
    let pool = migrated_pool().await;
    let movie = test_movie("Heat");

    SqliteMovieRepository::new(pool.clone())
        .delete_movie(movie.id(), &[movie_deleted(&movie)])
        .await
        .unwrap();

    assert_eq!(count(&pool, "event_queue").await, 0);
}

fn wrapup_record(user_id: Option<uuid::Uuid>) -> domain::models::wrapup::WrapUpRecord {
    use domain::models::wrapup::WrapUpStatus;
    domain::models::wrapup::WrapUpRecord {
        id: domain::value_objects::WrapUpId::generate(),
        user_id,
        start_date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        end_date: chrono::NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
        status: WrapUpStatus::Pending,
        report: None,
        error_message: None,
        created_at: chrono::Utc::now().naive_utc(),
        completed_at: None,
    }
}

fn wrapup_requested(record: &domain::models::wrapup::WrapUpRecord) -> DomainEvent {
    DomainEvent::WrapUpRequested {
        wrapup_id: record.id.clone(),
        user_id: None,
        start_date: record.start_date,
        end_date: record.end_date,
    }
}

fn wrapup_completed(record: &domain::models::wrapup::WrapUpRecord) -> DomainEvent {
    DomainEvent::WrapUpCompleted {
        wrapup_id: record.id.clone(),
    }
}

fn empty_report() -> domain::models::wrapup::WrapUpReport {
    serde_json::from_value(serde_json::json!({
        "scope": "Global",
        "date_range": { "start": "2024-01-01", "end": "2024-12-31" },
        "generated_at": "2025-01-01T00:00:00Z",
        "total_movies": 0,
        "total_watch_time_minutes": 0,
        "movies_per_month": [],
        "rating_distribution": [0, 0, 0, 0, 0],
        "top_directors": [],
        "top_actors": [],
        "director_diversity": 0,
        "actor_diversity": 0,
        "top_genres": [],
        "genre_diversity": 0,
        "top_keywords": [],
        "language_distribution": [],
        "watch_medium_distribution": [],
        "total_rewatches": 0,
        "poster_paths": [],
        "top_cast_profile_paths": [],
    }))
    .expect("minimal wrap-up report")
}

#[tokio::test]
async fn creating_a_wrapup_writes_the_record_and_its_event_in_one_commit() {
    let pool = migrated_pool().await;
    let record = wrapup_record(None);

    sqlite::SqliteWrapUpRepository::new(pool.clone())
        .create(&record, &[wrapup_requested(&record)])
        .await
        .unwrap();

    assert_eq!(count(&pool, "wrap_up_records").await, 1);
    assert_eq!(count(&pool, "event_queue").await, 1);
}

#[tokio::test]
async fn a_failing_enqueue_rolls_back_the_wrapup_create() {
    let pool = migrated_pool().await;
    let record = wrapup_record(None);
    fail_event_inserts(&pool).await;

    let result = sqlite::SqliteWrapUpRepository::new(pool.clone())
        .create(&record, &[wrapup_requested(&record)])
        .await;

    assert!(result.is_err());
    assert_eq!(count(&pool, "wrap_up_records").await, 0);
}

#[tokio::test]
async fn completing_a_wrapup_writes_the_report_and_its_event_in_one_commit() {
    let pool = migrated_pool().await;
    let repo = sqlite::SqliteWrapUpRepository::new(pool.clone());
    let record = wrapup_record(None);
    repo.create(&record, &[]).await.unwrap();

    repo.set_complete(&record.id, &empty_report(), &[wrapup_completed(&record)])
        .await
        .unwrap();

    let status: String = sqlx::query_scalar("SELECT status FROM wrap_up_records")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "ready");
    assert_eq!(count(&pool, "event_queue").await, 1);
}

#[tokio::test]
async fn a_failing_enqueue_rolls_back_the_wrapup_completion() {
    let pool = migrated_pool().await;
    let repo = sqlite::SqliteWrapUpRepository::new(pool.clone());
    let record = wrapup_record(None);
    repo.create(&record, &[]).await.unwrap();
    fail_event_inserts(&pool).await;

    let result = repo
        .set_complete(&record.id, &empty_report(), &[wrapup_completed(&record)])
        .await;

    assert!(result.is_err());
    let status: String = sqlx::query_scalar("SELECT status FROM wrap_up_records")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "pending", "completion rolled back");
}

#[tokio::test]
async fn completing_a_missing_wrapup_is_not_found_and_enqueues_no_event() {
    let pool = migrated_pool().await;
    let record = wrapup_record(None);

    let result = sqlite::SqliteWrapUpRepository::new(pool.clone())
        .set_complete(&record.id, &empty_report(), &[wrapup_completed(&record)])
        .await;

    assert!(matches!(
        result,
        Err(domain::errors::DomainError::NotFound(_))
    ));
    assert_eq!(count(&pool, "event_queue").await, 0);
}

fn watch_event_for(user_id: &UserId) -> domain::models::WatchEvent {
    domain::models::WatchEvent::new(
        user_id.clone(),
        "Heat".into(),
        Some(1995),
        None,
        domain::models::WatchEventSource::Jellyfin,
        chrono::Utc::now().naive_utc(),
        None,
    )
}

fn watch_event_ingested(user_id: &UserId) -> DomainEvent {
    DomainEvent::WatchEventIngested {
        user_id: user_id.clone(),
        title: "Heat".into(),
        source: "jellyfin".into(),
    }
}

#[tokio::test]
async fn saving_a_watch_event_writes_the_row_and_its_event_in_one_commit() {
    let (pool, user) = pool_with_user().await;
    let event = watch_event_for(user.id());

    sqlite::SqliteWatchEventRepository::new(pool.clone())
        .save(&event, &[watch_event_ingested(user.id())])
        .await
        .unwrap();

    assert_eq!(count(&pool, "watch_events").await, 1);
    assert_eq!(count(&pool, "event_queue").await, 1);
}

#[tokio::test]
async fn a_failing_enqueue_rolls_back_the_watch_event_save() {
    let (pool, user) = pool_with_user().await;
    let event = watch_event_for(user.id());
    fail_event_inserts(&pool).await;

    let result = sqlite::SqliteWatchEventRepository::new(pool.clone())
        .save(&event, &[watch_event_ingested(user.id())])
        .await;

    assert!(result.is_err());
    assert_eq!(count(&pool, "watch_events").await, 0);
}

fn follow_writes(user_id: &UserId) -> Vec<domain::ports::FollowWrite> {
    use domain::{ports::FollowWrite, value_objects::FollowStatus};
    vec![
        FollowWrite::AddFollower {
            local_user_id: user_id.value(),
            follower_actor_url: "https://md.example/users/other".into(),
            status: FollowStatus::Pending,
        },
        FollowWrite::AddFollow {
            follower_id: user_id.value(),
            target_actor_url: "https://md.example/users/other".into(),
            status: FollowStatus::Pending,
        },
    ]
}

fn follow_requested(user_id: &UserId) -> DomainEvent {
    DomainEvent::FollowRequested {
        follower: user_id.clone(),
        target: domain::value_objects::FollowTarget::Handle("@other@md.example".into()),
    }
}

fn social_repo(pool: &SqlitePool) -> sqlite::SqliteSocialRepository {
    sqlite::SqliteSocialRepository::new(
        pool.clone(),
        domain::value_objects::InstanceIdentity::new("https://md.example"),
    )
}

#[tokio::test]
async fn a_local_follow_writes_both_rows_and_its_event_in_one_commit() {
    use domain::ports::FollowCommand;
    let (pool, user) = pool_with_user().await;

    social_repo(&pool)
        .apply(&follow_writes(user.id()), &[follow_requested(user.id())])
        .await
        .unwrap();

    assert_eq!(count(&pool, "ap_followers").await, 1);
    assert_eq!(count(&pool, "ap_following").await, 1);
    assert_eq!(count(&pool, "event_queue").await, 1);
}

#[tokio::test]
async fn a_failing_enqueue_rolls_back_both_follow_rows() {
    use domain::ports::FollowCommand;
    let (pool, user) = pool_with_user().await;
    fail_event_inserts(&pool).await;

    let result = social_repo(&pool)
        .apply(&follow_writes(user.id()), &[follow_requested(user.id())])
        .await;

    assert!(result.is_err());
    assert_eq!(count(&pool, "ap_followers").await, 0);
    assert_eq!(count(&pool, "ap_following").await, 0);
}

#[tokio::test]
async fn a_failing_follow_write_rolls_back_the_rows_already_applied() {
    use domain::ports::{FollowCommand, FollowWrite};
    let (pool, user) = pool_with_user().await;
    let mut writes = follow_writes(user.id());
    writes.push(FollowWrite::AddFollow {
        follower_id: uuid::Uuid::new_v4(),
        target_actor_url: "https://md.example/users/other".into(),
        status: domain::value_objects::FollowStatus::Pending,
    });

    let result = social_repo(&pool)
        .apply(&writes, &[follow_requested(user.id())])
        .await;

    assert!(result.is_err());
    assert_eq!(count(&pool, "ap_followers").await, 0);
    assert_eq!(count(&pool, "event_queue").await, 0);
}
