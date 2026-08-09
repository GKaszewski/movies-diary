use std::sync::Arc;

use application::config::AppConfig;
use domain::testing::{
    FakeAuthService, FakeDocumentParser, FakeMetadataClient, FakePasswordHasher, FakePersonQuery,
    FakePosterFetcher, FakeSearchCommand, FakeSearchPort, FakeStatsRepository,
    InMemoryGoalRepository, InMemoryImportProfileRepository, InMemoryImportSessionRepository,
    InMemoryMovieProfileRepository, InMemoryMovieRepository, InMemoryProfileFieldsRepo,
    InMemoryRefreshSessionRepository, InMemoryReviewRepository, InMemorySocialRepository,
    InMemoryUserRepository, InMemoryUserSettingsRepository, InMemoryWatchEventRepository,
    InMemoryWatchlistRepository, InMemoryWebhookTokenRepository, InMemoryWrapUpRepository,
    InMemoryWrapUpStatsQuery, NoopEventPublisher, NoopFederationAdminQuery, NoopObjectStorage,
    NoopRemoteGoalRepository, NoopRemoteWatchlistRepository, PanicDiaryExporter,
    PanicPersonCommand,
};

use crate::{DatabaseOutput, Repositories, build::build_deps, build::build_worker_deps};

struct NoopReviewLogger;

#[async_trait::async_trait]
impl application::ports::ReviewLogger for NoopReviewLogger {
    async fn log_review(
        &self,
        _cmd: application::diary::commands::LogReviewCommand,
    ) -> Result<(), domain::errors::DomainError> {
        Ok(())
    }
}

struct NoopMovieDeduplicator;

#[async_trait::async_trait]
impl domain::ports::MovieDeduplicator for NoopMovieDeduplicator {
    async fn merge_into_canonical(
        &self,
        _old_id: &domain::value_objects::MovieId,
        _canonical: &domain::models::Movie,
    ) -> Result<u64, domain::errors::DomainError> {
        Ok(0)
    }
}

struct NoopLocalApContentQuery;

#[async_trait::async_trait]
impl domain::ports::LocalApContentQuery for NoopLocalApContentQuery {
    async fn get_local_watchlist_for_user(
        &self,
        _user_id: &domain::value_objects::UserId,
    ) -> Result<Vec<domain::models::WatchlistWithMovie>, domain::errors::DomainError> {
        Ok(vec![])
    }

    async fn get_local_reviews_for_movie(
        &self,
        _movie_id: &domain::value_objects::MovieId,
    ) -> Result<Vec<domain::models::DiaryEntry>, domain::errors::DomainError> {
        Ok(vec![])
    }

    async fn get_local_reviews_page(
        &self,
        _user_id: &domain::value_objects::UserId,
        _before: Option<chrono::NaiveDateTime>,
        _limit: usize,
    ) -> Result<Vec<domain::models::DiaryEntry>, domain::errors::DomainError> {
        Ok(vec![])
    }
}

struct NoopImageRefCommand;

#[async_trait::async_trait]
impl domain::ports::ImageRefCommand for NoopImageRefCommand {
    async fn swap(
        &self,
        _old_key: &str,
        _new_key: &str,
    ) -> Result<(), domain::errors::DomainError> {
        Ok(())
    }
}

struct NoopImageRefQuery;

#[async_trait::async_trait]
impl domain::ports::ImageRefQuery for NoopImageRefQuery {
    async fn list_keys(&self) -> Result<Vec<String>, domain::errors::DomainError> {
        Ok(vec![])
    }
}

/// `DatabaseOutput::follow_command`/`follow_query` need a fake independent of
/// `InMemorySocialRepository` (domain::testing), which implements
/// `SocialCommand`/`FollowGraphQuery`/`BlockQuery` — the composite ports these
/// two feed, one level up — so it cannot stand in as their dependency. These
/// fixture fields are never exercised by `build_deps`/`build_worker_deps`
/// (neither wires them further), so the bodies just need to type-check.
struct NoopFollowCommand;

#[async_trait::async_trait]
impl domain::ports::FollowCommand for NoopFollowCommand {
    async fn add_follow(
        &self,
        _follower_id: uuid::Uuid,
        _target_actor_url: &str,
        _status: domain::value_objects::FollowStatus,
    ) -> Result<(), domain::errors::DomainError> {
        Ok(())
    }

    async fn update_follow_status(
        &self,
        _follower_id: uuid::Uuid,
        _target_actor_url: &str,
        _status: domain::value_objects::FollowStatus,
    ) -> Result<(), domain::errors::DomainError> {
        Ok(())
    }

    async fn remove_follow(
        &self,
        _follower_id: uuid::Uuid,
        _target_actor_url: &str,
    ) -> Result<(), domain::errors::DomainError> {
        Ok(())
    }

    async fn add_follower(
        &self,
        _local_user_id: uuid::Uuid,
        _follower_actor_url: &str,
        _status: domain::value_objects::FollowStatus,
    ) -> Result<(), domain::errors::DomainError> {
        Ok(())
    }

    async fn update_follower_status(
        &self,
        _local_user_id: uuid::Uuid,
        _follower_actor_url: &str,
        _status: domain::value_objects::FollowStatus,
    ) -> Result<(), domain::errors::DomainError> {
        Ok(())
    }

    async fn remove_follower_record(
        &self,
        _local_user_id: uuid::Uuid,
        _follower_actor_url: &str,
    ) -> Result<(), domain::errors::DomainError> {
        Ok(())
    }
}

struct NoopFollowQuery;

#[async_trait::async_trait]
impl domain::ports::FollowQuery for NoopFollowQuery {
    async fn get_following(
        &self,
        _user_id: uuid::Uuid,
    ) -> Result<Vec<domain::value_objects::SocialActor>, domain::errors::DomainError> {
        Ok(vec![])
    }

    async fn get_followers(
        &self,
        _user_id: uuid::Uuid,
    ) -> Result<Vec<domain::value_objects::SocialActor>, domain::errors::DomainError> {
        Ok(vec![])
    }

    async fn get_pending_followers(
        &self,
        _user_id: uuid::Uuid,
    ) -> Result<Vec<domain::value_objects::SocialActor>, domain::errors::DomainError> {
        Ok(vec![])
    }

    async fn get_pending_following(
        &self,
        _user_id: uuid::Uuid,
    ) -> Result<Vec<domain::value_objects::SocialActor>, domain::errors::DomainError> {
        Ok(vec![])
    }

    async fn count_following(
        &self,
        _user_id: uuid::Uuid,
    ) -> Result<usize, domain::errors::DomainError> {
        Ok(0)
    }

    async fn count_followers(
        &self,
        _user_id: uuid::Uuid,
    ) -> Result<usize, domain::errors::DomainError> {
        Ok(0)
    }

    async fn count_pending_followers(
        &self,
        _user_id: uuid::Uuid,
    ) -> Result<usize, domain::errors::DomainError> {
        Ok(0)
    }

    async fn get_relation(
        &self,
        _viewer_id: uuid::Uuid,
        _target_actor_url: &str,
    ) -> Result<domain::value_objects::FollowRelation, domain::errors::DomainError> {
        // Empty/no-edge, matching every sibling method's empty-success shape above —
        // unlike them this isn't exercised by any test, so there's no behavior to pin,
        // just consistency for a future reader.
        Ok(domain::value_objects::FollowRelation {
            following: None,
            followed_by: None,
        })
    }
}

fn test_repositories() -> Repositories {
    let movies = InMemoryMovieRepository::new();
    let watch_events = InMemoryWatchEventRepository::new();
    let goals = InMemoryGoalRepository::new();
    let social = InMemorySocialRepository::new();

    Repositories {
        movie_command: Arc::clone(&movies) as _,
        movie_query: movies as _,
        review: InMemoryReviewRepository::new(),
        diary: domain::testing::FakeDiaryQuery::new(),
        stats: FakeStatsRepository::new(),
        user: InMemoryUserRepository::new(),
        import_session: InMemoryImportSessionRepository::new(),
        import_profile: InMemoryImportProfileRepository::new(),
        movie_profile: InMemoryMovieProfileRepository::new(),
        watchlist: InMemoryWatchlistRepository::new(),
        watch_event_command: Arc::clone(&watch_events) as _,
        watch_event_query: watch_events as _,
        webhook_token: InMemoryWebhookTokenRepository::new(),
        person_command: Arc::new(PanicPersonCommand),
        person_query: Arc::new(FakePersonQuery),
        search_port: Arc::new(FakeSearchPort),
        search_command: Arc::new(FakeSearchCommand),
        profile_fields: InMemoryProfileFieldsRepo::new(),
        remote_watchlist: Arc::new(NoopRemoteWatchlistRepository),
        social_command: Arc::clone(&social) as _,
        follow_graph: Arc::clone(&social) as _,
        block_query: Arc::clone(&social) as _,
        federation_admin: Arc::new(NoopFederationAdminQuery),
        wrapup_stats: InMemoryWrapUpStatsQuery::new(),
        wrapup_repo: InMemoryWrapUpRepository::new(),
        goal_command: Arc::clone(&goals) as _,
        goal_query: goals as _,
        user_settings: InMemoryUserSettingsRepository::new(),
        remote_goal: Arc::new(NoopRemoteGoalRepository),
        refresh_session: InMemoryRefreshSessionRepository::new(),
        federated_profile: None,
    }
}

/// A lazily-connected in-memory sqlite pool — never actually opened in this test,
/// just a value to satisfy `DatabaseOutput::db_pool`. Requires the `sqlite` feature,
/// which `cargo test --workspace` enables transitively (presentation/worker both
/// default to it), matching how `DbPool::Sqlite` is only constructible under that
/// feature everywhere else in the workspace.
#[cfg(feature = "sqlite")]
fn test_db_pool() -> infra_wiring::DbPool {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect_lazy("sqlite::memory:")
        .expect("lazy sqlite pool");
    infra_wiring::DbPool::Sqlite(pool)
}

#[cfg(not(feature = "sqlite"))]
fn test_db_pool() -> infra_wiring::DbPool {
    unreachable!(
        "worker_deps_wiring needs the sqlite feature; cargo test --workspace enables it \
         transitively via presentation/worker's default features"
    )
}

fn test_database_output() -> DatabaseOutput {
    let movies = InMemoryMovieRepository::new();
    let watch_events = InMemoryWatchEventRepository::new();
    let goals = InMemoryGoalRepository::new();
    let user_settings = InMemoryUserSettingsRepository::new();

    DatabaseOutput {
        movie_command: Arc::clone(&movies) as _,
        movie_query: movies as _,
        review: InMemoryReviewRepository::new(),
        diary: domain::testing::FakeDiaryQuery::new(),
        stats: FakeStatsRepository::new(),
        user: InMemoryUserRepository::new(),
        import_session: InMemoryImportSessionRepository::new(),
        import_profile: InMemoryImportProfileRepository::new(),
        movie_profile: InMemoryMovieProfileRepository::new(),
        watchlist: InMemoryWatchlistRepository::new(),
        watch_event_command: Arc::clone(&watch_events) as _,
        watch_event_query: watch_events as _,
        webhook_token: InMemoryWebhookTokenRepository::new(),
        person_command: Arc::new(PanicPersonCommand),
        person_query: Arc::new(FakePersonQuery),
        search_port: Arc::new(FakeSearchPort),
        search_command: Arc::new(FakeSearchCommand),
        profile_fields: InMemoryProfileFieldsRepo::new(),
        ap_content: Arc::new(NoopLocalApContentQuery),
        wrapup_stats: InMemoryWrapUpStatsQuery::new(),
        wrapup_repo: InMemoryWrapUpRepository::new(),
        goal_command: Arc::clone(&goals) as _,
        goal_query: goals as _,
        user_settings: Arc::clone(&user_settings) as _,
        federation_settings: user_settings as _,
        remote_goal: Arc::new(NoopRemoteGoalRepository),
        refresh_session: InMemoryRefreshSessionRepository::new(),
        deduplicator: Arc::new(NoopMovieDeduplicator),
        image_ref_command: Arc::new(NoopImageRefCommand),
        image_ref_query: Arc::new(NoopImageRefQuery),
        follow_command: Arc::new(NoopFollowCommand),
        follow_query: Arc::new(NoopFollowQuery),
        db_pool: test_db_pool(),
    }
}

fn test_services() -> application::Services {
    application::Services {
        auth: Arc::new(FakeAuthService),
        password_hasher: Arc::new(FakePasswordHasher),
        metadata: Arc::new(FakeMetadataClient),
        poster_fetcher: Arc::new(FakePosterFetcher),
        object_storage: Arc::new(NoopObjectStorage),
        event_publisher: NoopEventPublisher::new(),
        diary_exporter: Arc::new(PanicDiaryExporter),
        document_parser: Arc::new(FakeDocumentParser),
        review_logger: Arc::new(NoopReviewLogger),
        person_enrichment: None,
    }
}

fn test_worker_services() -> application::WorkerServices {
    application::WorkerServices {
        object_storage: Arc::new(NoopObjectStorage),
        event_publisher: NoopEventPublisher::new(),
        person_enrichment: None,
    }
}

fn test_config() -> AppConfig {
    AppConfig {
        allow_registration: true,
        base_url: "http://localhost:3000".into(),
        rate_limit: 20,
        refresh_ttl_seconds: 2_592_000,
        wrapup: infra_wiring::WrapUpConfig {
            font_path: None,
            logo_path: None,
            bg_dir: None,
        },
    }
}

/// The composition root must produce a Deps whose narrow structs point at the
/// same Arcs it was given. Proven by pointer identity on one representative
/// field per group boundary — if wiring is crossed, these differ.
#[test]
fn build_deps_wires_the_given_arcs() {
    let repos = test_repositories();
    let services = test_services();
    let config = test_config();
    let instance = domain::value_objects::InstanceIdentity::new("https://md.test");

    let deps = build_deps(&repos, &services, &config, &instance);

    assert!(
        std::sync::Arc::ptr_eq(&deps.users.get_local_profile.user, &repos.user),
        "get_local_profile.user must be the repositories' user repo"
    );
    assert!(
        std::sync::Arc::ptr_eq(&deps.social.query.follow_graph, &repos.follow_graph),
        "social.query.follow_graph must be the repositories' follow_graph"
    );
    assert_eq!(deps.users.get_local_profile.instance, instance);

    assert!(
        std::sync::Arc::ptr_eq(
            &deps.social.command.event_publisher,
            &services.event_publisher
        ),
        "social.command.event_publisher must be the services' event_publisher"
    );
}

// The test above covers one representative field per group boundary. The tests below
// give every one of the 11 groups its own coverage: at least one `Arc::ptr_eq` per
// struct in the group, favoring fields most likely to get silently crossed in a future
// edit — same-prefixed fields reused across sibling structs in the group (e.g.
// `wrapup_repo`/`wrapup_stats`, `watch_event_command`/`watch_event_query`), and the two
// name-mismatched `social_query` fields that read like they should come from a
// `social_query` repo but must come from `repos.follow_graph`.

#[test]
fn auth_group_wiring() {
    let repos = test_repositories();
    let services = test_services();
    let config = test_config();
    let instance = domain::value_objects::InstanceIdentity::new("https://md.test");
    let deps = build_deps(&repos, &services, &config, &instance);

    assert!(Arc::ptr_eq(&deps.auth.login.user, &repos.user));
    assert!(Arc::ptr_eq(&deps.auth.login.auth, &services.auth));
    assert!(Arc::ptr_eq(
        &deps.auth.login.refresh_session,
        &repos.refresh_session
    ));
    assert!(Arc::ptr_eq(
        &deps.auth.register.password_hasher,
        &services.password_hasher
    ));
    // login and refresh both carry refresh_session + auth — confirm the second struct
    // gets the same repo/service Arcs independently, not a stray clone of something else.
    assert!(Arc::ptr_eq(
        &deps.auth.refresh.refresh_session,
        &repos.refresh_session
    ));
    assert!(Arc::ptr_eq(&deps.auth.refresh.auth, &services.auth));
    assert!(Arc::ptr_eq(&deps.auth.register_and_login.user, &repos.user));
    assert_eq!(deps.auth.login.config.base_url, config.base_url);
    // logout shares refresh_session with login/refresh — confirm it's wired
    // independently to the same repo Arc.
    assert!(Arc::ptr_eq(
        &deps.auth.logout.refresh_session,
        &repos.refresh_session
    ));
}

#[test]
fn diary_group_wiring() {
    let repos = test_repositories();
    let services = test_services();
    let config = test_config();
    let instance = domain::value_objects::InstanceIdentity::new("https://md.test");
    let deps = build_deps(&repos, &services, &config, &instance);

    assert!(Arc::ptr_eq(&deps.diary.delete_review.review, &repos.review));
    assert!(Arc::ptr_eq(&deps.diary.delete_review.diary, &repos.diary));
    assert!(Arc::ptr_eq(
        &deps.diary.delete_review.movie_command,
        &repos.movie_command
    ));
    assert!(Arc::ptr_eq(
        &deps.diary.delete_review.event_publisher,
        &services.event_publisher
    ));
    // edit_review also carries `review` — confirm it's independently wired, not left
    // pointing at delete_review's clone by accident.
    assert!(Arc::ptr_eq(&deps.diary.edit_review.review, &repos.review));
    assert!(Arc::ptr_eq(
        &deps.diary.get_movie_social_page.movie_query,
        &repos.movie_query
    ));
    assert!(Arc::ptr_eq(
        &deps.diary.get_movie_social_page.movie_profile,
        &repos.movie_profile
    ));
    // Name mismatch: the field is called `social_query` but there is no
    // `repos.social_query` — it must come from `repos.follow_graph`.
    assert!(Arc::ptr_eq(
        &deps.diary.get_activity_feed.social_query,
        &repos.follow_graph
    ));
    assert_eq!(
        deps.diary.get_activity_feed.config.base_url,
        config.base_url
    );
    assert!(Arc::ptr_eq(&deps.diary.get_user_feed.user, &repos.user));
    assert!(Arc::ptr_eq(&deps.diary.get_user_feed.diary, &repos.diary));
    assert!(Arc::ptr_eq(&deps.diary.get_diary.diary, &repos.diary));
    assert!(Arc::ptr_eq(
        &deps.diary.get_review_history.diary,
        &repos.diary
    ));
    assert!(Arc::ptr_eq(&deps.diary.export_diary.diary, &repos.diary));
    assert!(Arc::ptr_eq(
        &deps.diary.export_diary.diary_exporter,
        &services.diary_exporter
    ));
}

#[test]
fn goals_group_wiring() {
    let repos = test_repositories();
    let services = test_services();
    let config = test_config();
    let instance = domain::value_objects::InstanceIdentity::new("https://md.test");
    let deps = build_deps(&repos, &services, &config, &instance);

    assert!(Arc::ptr_eq(
        &deps.goals.command.goal_command,
        &repos.goal_command
    ));
    assert!(Arc::ptr_eq(
        &deps.goals.command.event_publisher,
        &services.event_publisher
    ));
    // goal_query and stats each appear in both command and query — confirm both
    // structs land on the same underlying repo Arc rather than diverging.
    assert!(Arc::ptr_eq(
        &deps.goals.command.goal_query,
        &repos.goal_query
    ));
    assert!(Arc::ptr_eq(&deps.goals.query.goal_query, &repos.goal_query));
    assert!(Arc::ptr_eq(&deps.goals.command.stats, &repos.stats));
    assert!(Arc::ptr_eq(&deps.goals.query.stats, &repos.stats));
}

#[test]
fn import_group_wiring() {
    let repos = test_repositories();
    let services = test_services();
    let config = test_config();
    let instance = domain::value_objects::InstanceIdentity::new("https://md.test");
    let deps = build_deps(&repos, &services, &config, &instance);

    // document_parser (a service) is shared by create_session and apply_mapping —
    // confirm both point at the same Arc, not two independently-sourced ones.
    assert!(Arc::ptr_eq(
        &deps.import.create_session.document_parser,
        &services.document_parser
    ));
    assert!(Arc::ptr_eq(
        &deps.import.apply_mapping.document_parser,
        &services.document_parser
    ));
    assert!(Arc::ptr_eq(
        &deps.import.apply_mapping.movie_query,
        &repos.movie_query
    ));
    // import_session/import_profile recur across apply_profile and save_profile —
    // confirm both fields land correctly in both structs.
    assert!(Arc::ptr_eq(
        &deps.import.apply_profile.import_profile,
        &repos.import_profile
    ));
    assert!(Arc::ptr_eq(
        &deps.import.apply_profile.import_session,
        &repos.import_session
    ));
    assert!(Arc::ptr_eq(
        &deps.import.save_profile.import_session,
        &repos.import_session
    ));
    assert!(Arc::ptr_eq(
        &deps.import.save_profile.import_profile,
        &repos.import_profile
    ));
    assert!(Arc::ptr_eq(
        &deps.import.execute_import.review_logger,
        &services.review_logger
    ));

    // The four stage-gating/orchestration use cases added alongside the
    // handler-bypass removal — each field asserted individually since several
    // recur across siblings above (import_session, movie_query, document_parser).
    assert!(Arc::ptr_eq(
        &deps.import.get_mapping_stage.import_session,
        &repos.import_session
    ));
    assert!(Arc::ptr_eq(
        &deps.import.get_preview_stage.import_session,
        &repos.import_session
    ));
    assert!(Arc::ptr_eq(
        &deps.import.get_session_state.import_session,
        &repos.import_session
    ));
    assert!(Arc::ptr_eq(
        &deps.import.apply_profile_and_map.import_profile,
        &repos.import_profile
    ));
    assert!(Arc::ptr_eq(
        &deps.import.apply_profile_and_map.import_session,
        &repos.import_session
    ));
    assert!(Arc::ptr_eq(
        &deps.import.apply_profile_and_map.document_parser,
        &services.document_parser
    ));
    assert!(Arc::ptr_eq(
        &deps.import.apply_profile_and_map.movie_query,
        &repos.movie_query
    ));
    // import_profile recurs again here — confirm both new use cases are wired
    // independently rather than sharing apply_profile's/save_profile's clone.
    assert!(Arc::ptr_eq(
        &deps.import.delete_profile.import_profile,
        &repos.import_profile
    ));
    assert!(Arc::ptr_eq(
        &deps.import.list_profiles.import_profile,
        &repos.import_profile
    ));
}

#[test]
fn integrations_group_wiring() {
    let repos = test_repositories();
    let services = test_services();
    let config = test_config();
    let instance = domain::value_objects::InstanceIdentity::new("https://md.test");
    let deps = build_deps(&repos, &services, &config, &instance);

    // watch_event_command and watch_event_query are the classic same-prefix pair most
    // at risk of being swapped by a future reader.
    assert!(Arc::ptr_eq(
        &deps.integrations.ingest_watch_event.watch_event_command,
        &repos.watch_event_command
    ));
    assert!(Arc::ptr_eq(
        &deps.integrations.ingest_watch_event.watch_event_query,
        &repos.watch_event_query
    ));
    assert!(Arc::ptr_eq(
        &deps.integrations.ingest_watch_event.webhook_token,
        &repos.webhook_token
    ));
    assert!(Arc::ptr_eq(
        &deps.integrations.ingest_watch_event.event_publisher,
        &services.event_publisher
    ));
    assert!(Arc::ptr_eq(
        &deps.integrations.confirm_watch_events.watch_event_command,
        &repos.watch_event_command
    ));
    assert!(Arc::ptr_eq(
        &deps.integrations.confirm_watch_events.watch_event_query,
        &repos.watch_event_query
    ));
    assert!(Arc::ptr_eq(
        &deps.integrations.confirm_watch_events.review_logger,
        &services.review_logger
    ));
    assert!(Arc::ptr_eq(
        &deps.integrations.dismiss_watch_events.watch_event_command,
        &repos.watch_event_command
    ));
    assert!(Arc::ptr_eq(
        &deps.integrations.dismiss_watch_events.watch_event_query,
        &repos.watch_event_query
    ));
    assert!(Arc::ptr_eq(
        &deps.integrations.generate_webhook_token.webhook_token,
        &repos.webhook_token
    ));
    assert!(Arc::ptr_eq(
        &deps.integrations.get_watch_queue.watch_event_query,
        &repos.watch_event_query
    ));
    assert!(Arc::ptr_eq(
        &deps.integrations.get_webhook_tokens.webhook_token,
        &repos.webhook_token
    ));
    assert!(Arc::ptr_eq(
        &deps.integrations.revoke_webhook_token.webhook_token,
        &repos.webhook_token
    ));
}

/// MoviesGroup was the reviewer's worked example of the risk this suite closes: 8
/// same-shaped `.clone()` calls on one struct where a crossed pair would still compile.
/// Every field of `sync_poster` gets its own assertion for that reason.
#[test]
fn movies_group_wiring() {
    let repos = test_repositories();
    let services = test_services();
    let config = test_config();
    let instance = domain::value_objects::InstanceIdentity::new("https://md.test");
    let deps = build_deps(&repos, &services, &config, &instance);

    assert!(Arc::ptr_eq(
        &deps.movies.sync_poster.movie_command,
        &repos.movie_command
    ));
    assert!(Arc::ptr_eq(
        &deps.movies.sync_poster.movie_query,
        &repos.movie_query
    ));
    assert!(Arc::ptr_eq(
        &deps.movies.sync_poster.movie_profile,
        &repos.movie_profile
    ));
    assert!(Arc::ptr_eq(
        &deps.movies.sync_poster.metadata,
        &services.metadata
    ));
    assert!(Arc::ptr_eq(
        &deps.movies.sync_poster.poster_fetcher,
        &services.poster_fetcher
    ));
    assert!(Arc::ptr_eq(
        &deps.movies.sync_poster.object_storage,
        &services.object_storage
    ));
    assert!(Arc::ptr_eq(
        &deps.movies.sync_poster.event_publisher,
        &services.event_publisher
    ));
    assert!(Arc::ptr_eq(
        &deps.movies.sync_poster.search_command,
        &repos.search_command
    ));
    // movie_profile/movie_query also recur in sync_poster above — confirm these two
    // use cases are wired independently rather than reusing sync_poster's clone.
    assert!(Arc::ptr_eq(
        &deps.movies.get_movie_profile.movie_profile,
        &repos.movie_profile
    ));
    assert!(Arc::ptr_eq(
        &deps.movies.get_movies.movie,
        &repos.movie_query
    ));
}

#[test]
fn person_group_wiring() {
    let repos = test_repositories();
    let services = test_services();
    let config = test_config();
    let instance = domain::value_objects::InstanceIdentity::new("https://md.test");
    let deps = build_deps(&repos, &services, &config, &instance);

    assert!(Arc::ptr_eq(
        &deps.person.get_person.person_query,
        &repos.person_query
    ));
    assert!(Arc::ptr_eq(
        &deps.person.get_person.event_publisher,
        &services.event_publisher
    ));
}

#[test]
fn search_group_wiring() {
    let repos = test_repositories();
    let services = test_services();
    let config = test_config();
    let instance = domain::value_objects::InstanceIdentity::new("https://md.test");
    let deps = build_deps(&repos, &services, &config, &instance);

    assert!(Arc::ptr_eq(
        &deps.search.execute.search_port,
        &repos.search_port
    ));
}

#[test]
fn social_group_wiring() {
    let repos = test_repositories();
    let services = test_services();
    let config = test_config();
    let instance = domain::value_objects::InstanceIdentity::new("https://md.test");
    let deps = build_deps(&repos, &services, &config, &instance);

    assert!(Arc::ptr_eq(
        &deps.social.command.social_command,
        &repos.social_command
    ));
    assert!(Arc::ptr_eq(
        &deps.social.command.event_publisher,
        &services.event_publisher
    ));
    assert!(Arc::ptr_eq(
        &deps.social.query.follow_graph,
        &repos.follow_graph
    ));
    assert!(Arc::ptr_eq(
        &deps.social.query.block_query,
        &repos.block_query
    ));
}

#[test]
fn users_group_wiring() {
    let repos = test_repositories();
    let services = test_services();
    let config = test_config();
    let instance = domain::value_objects::InstanceIdentity::new("https://md.test");
    let deps = build_deps(&repos, &services, &config, &instance);

    assert!(Arc::ptr_eq(
        &deps.users.get_local_profile.stats,
        &repos.stats
    ));
    assert!(Arc::ptr_eq(
        &deps.users.get_local_profile.diary,
        &repos.diary
    ));
    // Name mismatch: the field is called `social_query` but must come from
    // `repos.follow_graph`, same as diary::get_activity_feed above.
    assert!(Arc::ptr_eq(
        &deps.users.get_local_profile.social_query,
        &repos.follow_graph
    ));
    assert_eq!(deps.users.get_local_profile.instance, instance);

    assert!(Arc::ptr_eq(
        &deps.users.get_federated_profile_stats.stats,
        &repos.stats
    ));
    assert!(Arc::ptr_eq(
        &deps.users.get_federated_profile_stats.diary,
        &repos.diary
    ));
    assert!(Arc::ptr_eq(
        &deps.users.get_federated_profile_stats.social_query,
        &repos.follow_graph
    ));

    assert!(Arc::ptr_eq(&deps.users.get_page_viewer.user, &repos.user));
    assert!(Arc::ptr_eq(
        &deps.users.get_page_viewer.follow_graph,
        &repos.follow_graph
    ));

    assert!(Arc::ptr_eq(&deps.users.resolve_username.user, &repos.user));

    assert!(Arc::ptr_eq(
        &deps.users.get_profile_settings.user,
        &repos.user
    ));
    assert!(Arc::ptr_eq(
        &deps.users.get_profile_settings.profile_fields,
        &repos.profile_fields
    ));
    assert_eq!(deps.users.get_profile_settings.instance, instance);

    // `user` recurs in several structs of this group — confirm each is wired
    // independently rather than one struct silently sharing another's clone call.
    assert!(Arc::ptr_eq(&deps.users.get_local_profile.user, &repos.user));
    assert!(Arc::ptr_eq(&deps.users.get_users_list.user, &repos.user));
    assert!(Arc::ptr_eq(&deps.users.update_profile.user, &repos.user));
    assert!(Arc::ptr_eq(&deps.users.delete_account.user, &repos.user));

    assert!(Arc::ptr_eq(
        &deps.users.get_users_list.federation_admin,
        &repos.federation_admin
    ));
    assert!(Arc::ptr_eq(
        &deps.users.update_profile.object_storage,
        &services.object_storage
    ));
    // event_publisher recurs in update_profile and delete_account — confirm both.
    assert!(Arc::ptr_eq(
        &deps.users.update_profile.event_publisher,
        &services.event_publisher
    ));
    assert!(Arc::ptr_eq(
        &deps.users.delete_account.event_publisher,
        &services.event_publisher
    ));

    // `user` recurs again here — confirm get_current_profile is independently wired.
    assert!(Arc::ptr_eq(
        &deps.users.get_current_profile.user,
        &repos.user
    ));
    assert!(Arc::ptr_eq(
        &deps.users.update_profile_fields.profile_fields,
        &repos.profile_fields
    ));
    assert!(Arc::ptr_eq(
        &deps.users.update_profile_fields.event_publisher,
        &services.event_publisher
    ));
    // user_settings recurs across get_settings and update_settings — confirm both
    // land on the same repo Arc independently.
    assert!(Arc::ptr_eq(
        &deps.users.get_settings.user_settings,
        &repos.user_settings
    ));
    assert!(Arc::ptr_eq(
        &deps.users.update_settings.user_settings,
        &repos.user_settings
    ));
    assert!(Arc::ptr_eq(&deps.users.authorize_admin.user, &repos.user));

    // federated_profile is genuinely optional configuration (federation on/off),
    // the same exemption as WorkerServices::person_enrichment (see
    // worker_deps_wiring below) — the fixture leaves it None, so pointer
    // identity doesn't apply; confirm it passes through unchanged instead.
    assert_eq!(
        deps.users.get_federated_profile.federated_profile.is_none(),
        repos.federated_profile.is_none(),
        "the optional federated_profile port must pass through as given"
    );
}

#[test]
fn watchlist_group_wiring() {
    let repos = test_repositories();
    let services = test_services();
    let config = test_config();
    let instance = domain::value_objects::InstanceIdentity::new("https://md.test");
    let deps = build_deps(&repos, &services, &config, &instance);

    assert!(Arc::ptr_eq(
        &deps.watchlist.add.movie_command,
        &repos.movie_command
    ));
    assert!(Arc::ptr_eq(
        &deps.watchlist.add.movie_query,
        &repos.movie_query
    ));
    assert!(Arc::ptr_eq(
        &deps.watchlist.add.metadata,
        &services.metadata
    ));
    assert!(Arc::ptr_eq(&deps.watchlist.add.watchlist, &repos.watchlist));
    assert!(Arc::ptr_eq(
        &deps.watchlist.add.event_publisher,
        &services.event_publisher
    ));
    assert!(Arc::ptr_eq(
        &deps.watchlist.get_watchlist_for_owner.user,
        &repos.user
    ));
    // `watchlist` also appears in `add` — confirm `get_watchlist_for_owner` is
    // independently wired to the same repo Arc, not a stray clone.
    assert!(Arc::ptr_eq(
        &deps.watchlist.get_watchlist_for_owner.watchlist,
        &repos.watchlist
    ));
    assert!(Arc::ptr_eq(
        &deps.watchlist.get_watchlist_for_owner.remote_watchlist,
        &repos.remote_watchlist
    ));
    assert!(Arc::ptr_eq(
        &deps.watchlist.get_watchlist.watchlist,
        &repos.watchlist
    ));
    assert!(Arc::ptr_eq(
        &deps.watchlist.is_on_watchlist.watchlist,
        &repos.watchlist
    ));
    assert!(Arc::ptr_eq(
        &deps.watchlist.remove_from_watchlist.watchlist,
        &repos.watchlist
    ));
    assert!(Arc::ptr_eq(
        &deps.watchlist.remove_from_watchlist.event_publisher,
        &services.event_publisher
    ));
}

#[test]
fn wrapup_group_wiring() {
    let repos = test_repositories();
    let services = test_services();
    let config = test_config();
    let instance = domain::value_objects::InstanceIdentity::new("https://md.test");
    let deps = build_deps(&repos, &services, &config, &instance);

    assert!(Arc::ptr_eq(
        &deps.wrapup.get_ready_report.wrapup_repo,
        &repos.wrapup_repo
    ));
    assert!(Arc::ptr_eq(
        &deps.wrapup.delete_wrapup.wrapup_repo,
        &repos.wrapup_repo
    ));
    assert!(Arc::ptr_eq(
        &deps.wrapup.generate.wrapup_repo,
        &repos.wrapup_repo
    ));
    assert!(Arc::ptr_eq(
        &deps.wrapup.generate.event_publisher,
        &services.event_publisher
    ));
    assert!(Arc::ptr_eq(
        &deps.wrapup.get_wrapup.wrapup_repo,
        &repos.wrapup_repo
    ));
    assert!(Arc::ptr_eq(
        &deps.wrapup.list_wrapups.wrapup_repo,
        &repos.wrapup_repo
    ));
}

/// Covers the five groups that moved to `WorkerDeps` during worker unification:
/// `enrich_movie`, `reindex_search`, `merge_duplicates`, `enrich_person`,
/// `handle_requested`. Assertions here are the ones that used to live in
/// `movies_group_wiring`, `person_group_wiring`, and `wrapup_group_wiring` against
/// `build_deps` — they now drive `build_worker_deps` against `WorkerServices` instead.
#[tokio::test]
async fn worker_deps_wiring() {
    let db = test_database_output();
    let services = test_worker_services();
    let worker_deps = build_worker_deps(&db, &services);

    // enrich_movie and reindex_search share movie_query/movie_profile/search_command
    // with each other — confirm each struct is wired independently rather than
    // reusing a stray clone from a sibling struct.
    assert!(Arc::ptr_eq(
        &worker_deps.enrich_movie.movie_query,
        &db.movie_query
    ));
    assert!(Arc::ptr_eq(
        &worker_deps.enrich_movie.movie_profile,
        &db.movie_profile
    ));
    assert!(Arc::ptr_eq(
        &worker_deps.enrich_movie.person_command,
        &db.person_command
    ));
    assert!(Arc::ptr_eq(
        &worker_deps.enrich_movie.search_command,
        &db.search_command
    ));
    assert!(Arc::ptr_eq(
        &worker_deps.reindex_search.movie_query,
        &db.movie_query
    ));
    assert!(Arc::ptr_eq(
        &worker_deps.reindex_search.movie_profile,
        &db.movie_profile
    ));
    assert!(Arc::ptr_eq(
        &worker_deps.reindex_search.search_command,
        &db.search_command
    ));
    assert!(Arc::ptr_eq(
        &worker_deps.reindex_search.person_command,
        &db.person_command
    ));
    assert!(Arc::ptr_eq(
        &worker_deps.reindex_search.person_query,
        &db.person_query
    ));

    assert!(Arc::ptr_eq(
        &worker_deps.merge_duplicates.movie_query,
        &db.movie_query
    ));
    assert!(Arc::ptr_eq(
        &worker_deps.merge_duplicates.object_storage,
        &services.object_storage
    ));
    assert!(
        Arc::ptr_eq(&worker_deps.merge_duplicates.deduplicator, &db.deduplicator),
        "merge_duplicates must carry the DatabaseOutput deduplicator — this field was \
         unconstructible before worker unification"
    );

    assert!(Arc::ptr_eq(
        &worker_deps.enrich_person.person_query,
        &db.person_query
    ));
    assert!(Arc::ptr_eq(
        &worker_deps.enrich_person.person_command,
        &db.person_command
    ));
    assert_eq!(
        worker_deps.enrich_person.person_enrichment.is_none(),
        services.person_enrichment.is_none(),
        "the optional person_enrichment port must pass through as given"
    );

    // wrapup_repo and wrapup_stats are the same-prefix pair most at risk of a swap;
    // both are different trait types, but a future reader "correcting" a name could
    // still reach for the wrong one.
    assert!(Arc::ptr_eq(
        &worker_deps.handle_requested.wrapup_repo,
        &db.wrapup_repo
    ));
    assert!(Arc::ptr_eq(
        &worker_deps.handle_requested.wrapup_stats,
        &db.wrapup_stats
    ));
    assert!(Arc::ptr_eq(
        &worker_deps.handle_requested.event_publisher,
        &services.event_publisher
    ));
}
