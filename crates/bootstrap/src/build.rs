use application::auth::deps::{
    LoginDeps, LogoutDeps, RefreshDeps, RegisterAndLoginDeps, RegisterDeps,
};
use application::deps::{
    AuthGroup, Deps, DiaryGroup, GoalsGroup, ImportGroup, InsightsGroup, IntegrationsGroup,
    MoviesGroup, PersonGroup, SearchGroup, SocialGroup, UsersGroup, WatchlistGroup, WorkerDeps,
    WrapupGroup,
};
use application::diary::deps::{
    DeleteReviewDeps, EditReviewDeps, ExportDiaryDeps, GetActivityFeedDeps, GetDiaryDeps,
    GetMovieSocialPageDeps, GetReviewHistoryDeps, GetUserFeedDeps,
};
use application::goals::deps::{GoalCommandDeps, GoalQueryDeps};
use application::import::deps::{
    ApplyMappingDeps, ApplyProfileAndMapDeps, ApplyProfileDeps, CreateSessionDeps,
    DeleteImportProfileDeps, ExecuteImportDeps, GetMappingStageDeps, GetPreviewStageDeps,
    GetSessionStateDeps, ListImportProfilesDeps, SaveProfileDeps,
};
use application::insights::deps::GetInsightsDeps;
use application::integrations::deps::{
    ConfirmWatchEventsDeps, DismissWatchEventsDeps, GenerateWebhookTokenDeps, GetWatchQueueDeps,
    GetWebhookTokensDeps, IngestWatchEventDeps, RevokeWebhookTokenDeps,
};
use application::movies::deps::{
    EnrichMovieDeps, GetMovieProfileDeps, GetMoviesDeps, ReindexSearchDeps, SyncPosterDeps,
};
use application::movies::merge_duplicates::MergeDuplicatesDeps;
use application::person::deps::{EnrichPersonDeps, GetPersonDeps};
use application::search::deps::SearchDeps;
use application::social::deps::{SocialCommandDeps, SocialQueryDeps};
use application::users::deps::{
    AuthorizeAdminDeps, EnsureActiveDeps, GetCurrentProfileDeps, GetFederatedProfileDeps,
    GetFederatedProfileStatsDeps, GetLocalProfileDeps, GetPageViewerDeps, GetProfileSettingsDeps,
    GetSettingsDeps, GetUsersListDeps, RequestAccountDeletionDeps, ResolveUsernameDeps,
    UpdateProfileDeps, UpdateProfileFieldsDeps, UpdateSettingsDeps,
};
use application::watchlist::deps::{
    GetWatchlistDeps, GetWatchlistForOwnerDeps, IsOnWatchlistDeps, RemoveFromWatchlistDeps,
    WatchlistAddDeps,
};
use application::wrapup::deps::{
    DeleteWrapUpDeps, GenerateWrapUpDeps, GetReadyReportDeps, GetWrapUpDeps,
    HandleWrapUpRequestedDeps, ListWrapUpsDeps,
};
use domain::value_objects::InstanceIdentity;

use crate::{Repositories, database::DatabaseOutput};

/// The composition root's single assembly point: every one of the server-facing
/// deps structs, built once from the already-constructed `Repositories` and
/// `application::Services`. Only `Arc` clones happen here — no adapters are built.
pub fn build_deps(
    repos: &Repositories,
    services: &application::Services,
    config: &application::config::AppSettings,
    instance: &InstanceIdentity,
) -> Deps {
    Deps {
        auth: AuthGroup {
            login: LoginDeps {
                user: repos.user.clone(),
                password_hasher: services.password_hasher.clone(),
                auth: services.auth.clone(),
                refresh_session: repos.refresh_session.clone(),
                config: config.clone(),
            },
            register: RegisterDeps {
                user: repos.user.clone(),
                password_hasher: services.password_hasher.clone(),
                config: config.clone(),
            },
            refresh: RefreshDeps {
                refresh_session: repos.refresh_session.clone(),
                auth: services.auth.clone(),
                config: config.clone(),
            },
            register_and_login: RegisterAndLoginDeps {
                user: repos.user.clone(),
                password_hasher: services.password_hasher.clone(),
                auth: services.auth.clone(),
                refresh_session: repos.refresh_session.clone(),
                config: config.clone(),
            },
            logout: LogoutDeps {
                refresh_session: repos.refresh_session.clone(),
            },
        },
        diary: DiaryGroup {
            delete_review: DeleteReviewDeps {
                review: repos.review.clone(),
                diary: repos.diary.clone(),
            },
            edit_review: EditReviewDeps {
                review: repos.review.clone(),
            },
            get_movie_social_page: GetMovieSocialPageDeps {
                movie_query: repos.movie_query.clone(),
                diary: repos.diary.clone(),
                movie_profile: repos.movie_profile.clone(),
            },
            get_activity_feed: GetActivityFeedDeps {
                diary: repos.diary.clone(),
                social_query: repos.follow_graph.clone(),
                config: config.clone(),
            },
            get_user_feed: GetUserFeedDeps {
                user: repos.user.clone(),
                diary: repos.diary.clone(),
            },
            get_diary: GetDiaryDeps {
                diary: repos.diary.clone(),
            },
            get_review_history: GetReviewHistoryDeps {
                diary: repos.diary.clone(),
            },
            export_diary: ExportDiaryDeps {
                diary: repos.diary.clone(),
                diary_exporter: services.diary_exporter.clone(),
            },
        },
        goals: GoalsGroup {
            command: GoalCommandDeps {
                goal_command: repos.goal_command.clone(),
                goal_query: repos.goal_query.clone(),
                stats: repos.stats.clone(),
            },
            query: GoalQueryDeps {
                goal_query: repos.goal_query.clone(),
                stats: repos.stats.clone(),
            },
        },
        import: ImportGroup {
            create_session: CreateSessionDeps {
                import_session: repos.import_session.clone(),
                document_parser: services.document_parser.clone(),
            },
            apply_mapping: ApplyMappingDeps {
                import_session: repos.import_session.clone(),
                document_parser: services.document_parser.clone(),
                movie_query: repos.movie_query.clone(),
            },
            apply_profile: ApplyProfileDeps {
                import_profile: repos.import_profile.clone(),
                import_session: repos.import_session.clone(),
            },
            execute_import: ExecuteImportDeps {
                import_session: repos.import_session.clone(),
                review_logger: services.review_logger.clone(),
            },
            save_profile: SaveProfileDeps {
                import_session: repos.import_session.clone(),
                import_profile: repos.import_profile.clone(),
            },
            get_mapping_stage: GetMappingStageDeps {
                import_session: repos.import_session.clone(),
            },
            get_preview_stage: GetPreviewStageDeps {
                import_session: repos.import_session.clone(),
            },
            get_session_state: GetSessionStateDeps {
                import_session: repos.import_session.clone(),
            },
            apply_profile_and_map: ApplyProfileAndMapDeps {
                import_profile: repos.import_profile.clone(),
                import_session: repos.import_session.clone(),
                document_parser: services.document_parser.clone(),
                movie_query: repos.movie_query.clone(),
            },
            delete_profile: DeleteImportProfileDeps {
                import_profile: repos.import_profile.clone(),
            },
            list_profiles: ListImportProfilesDeps {
                import_profile: repos.import_profile.clone(),
            },
        },
        insights: InsightsGroup {
            get: GetInsightsDeps {
                insights: repos.insights.clone(),
            },
        },
        integrations: IntegrationsGroup {
            ingest_watch_event: IngestWatchEventDeps {
                webhook_token: repos.webhook_token.clone(),
                watch_event_command: repos.watch_event_command.clone(),
                watch_event_query: repos.watch_event_query.clone(),
            },
            confirm_watch_events: ConfirmWatchEventsDeps {
                watch_event_command: repos.watch_event_command.clone(),
                watch_event_query: repos.watch_event_query.clone(),
                review_logger: services.review_logger.clone(),
            },
            dismiss_watch_events: DismissWatchEventsDeps {
                watch_event_command: repos.watch_event_command.clone(),
                watch_event_query: repos.watch_event_query.clone(),
            },
            generate_webhook_token: GenerateWebhookTokenDeps {
                webhook_token: repos.webhook_token.clone(),
            },
            get_watch_queue: GetWatchQueueDeps {
                watch_event_query: repos.watch_event_query.clone(),
            },
            get_webhook_tokens: GetWebhookTokensDeps {
                webhook_token: repos.webhook_token.clone(),
            },
            revoke_webhook_token: RevokeWebhookTokenDeps {
                webhook_token: repos.webhook_token.clone(),
            },
            jellyfin_parser: std::sync::Arc::new(media_server::JellyfinParser),
            plex_parser: std::sync::Arc::new(media_server::PlexParser),
        },
        movies: MoviesGroup {
            sync_poster: SyncPosterDeps {
                movie_command: repos.movie_command.clone(),
                movie_query: repos.movie_query.clone(),
                movie_profile: repos.movie_profile.clone(),
                metadata: services.metadata.clone(),
                poster_fetcher: services.poster_fetcher.clone(),
                object_storage: services.object_storage.clone(),
                event_publisher: services.event_publisher.clone(),
                search_command: repos.search_command.clone(),
            },
            get_movie_profile: GetMovieProfileDeps {
                movie_profile: repos.movie_profile.clone(),
            },
            get_movies: GetMoviesDeps {
                movie: repos.movie_query.clone(),
            },
        },
        person: PersonGroup {
            get_person: GetPersonDeps {
                person_query: repos.person_query.clone(),
                event_publisher: services.event_publisher.clone(),
            },
        },
        search: SearchGroup {
            execute: SearchDeps {
                search_port: repos.search_port.clone(),
            },
        },
        social: SocialGroup {
            command: SocialCommandDeps {
                social_command: repos.social_command.clone(),
            },
            query: SocialQueryDeps {
                follow_graph: repos.follow_graph.clone(),
                block_query: repos.block_query.clone(),
            },
        },
        users: UsersGroup {
            get_local_profile: GetLocalProfileDeps {
                stats: repos.stats.clone(),
                diary: repos.diary.clone(),
                social_query: repos.follow_graph.clone(),
                user: repos.user.clone(),
                instance: instance.clone(),
            },
            get_federated_profile_stats: GetFederatedProfileStatsDeps {
                stats: repos.stats.clone(),
                diary: repos.diary.clone(),
                social_query: repos.follow_graph.clone(),
            },
            get_page_viewer: GetPageViewerDeps {
                user: repos.user.clone(),
                follow_graph: repos.follow_graph.clone(),
            },
            resolve_username: ResolveUsernameDeps {
                user: repos.user.clone(),
            },
            get_profile_settings: GetProfileSettingsDeps {
                user: repos.user.clone(),
                profile_fields: repos.profile_fields.clone(),
                instance: instance.clone(),
            },
            get_users_list: GetUsersListDeps {
                user: repos.user.clone(),
                federation_admin: repos.federation_admin.clone(),
            },
            update_profile: UpdateProfileDeps {
                user: repos.user.clone(),
                object_storage: services.object_storage.clone(),
                event_publisher: services.event_publisher.clone(),
            },
            request_account_deletion: RequestAccountDeletionDeps {
                user: repos.user.clone(),
                password_hasher: services.password_hasher.clone(),
            },
            ensure_active: EnsureActiveDeps {
                user: repos.user.clone(),
            },
            get_current_profile: GetCurrentProfileDeps {
                user: repos.user.clone(),
            },
            update_profile_fields: UpdateProfileFieldsDeps {
                profile_fields: repos.profile_fields.clone(),
            },
            get_settings: GetSettingsDeps {
                user_settings: repos.user_settings.clone(),
            },
            update_settings: UpdateSettingsDeps {
                user_settings: repos.user_settings.clone(),
            },
            authorize_admin: AuthorizeAdminDeps {
                user: repos.user.clone(),
            },
            get_federated_profile: GetFederatedProfileDeps {
                federated_profile: repos.federated_profile.clone(),
            },
        },
        watchlist: WatchlistGroup {
            add: WatchlistAddDeps {
                movie_command: repos.movie_command.clone(),
                movie_query: repos.movie_query.clone(),
                metadata: services.metadata.clone(),
                watchlist: repos.watchlist.clone(),
            },
            get_watchlist_for_owner: GetWatchlistForOwnerDeps {
                user: repos.user.clone(),
                watchlist: repos.watchlist.clone(),
                remote_watchlist: repos.remote_watchlist.clone(),
            },
            get_watchlist: GetWatchlistDeps {
                watchlist: repos.watchlist.clone(),
            },
            is_on_watchlist: IsOnWatchlistDeps {
                watchlist: repos.watchlist.clone(),
            },
            remove_from_watchlist: RemoveFromWatchlistDeps {
                watchlist: repos.watchlist.clone(),
            },
        },
        wrapup: WrapupGroup {
            get_ready_report: GetReadyReportDeps {
                wrapup_repo: repos.wrapup_repo.clone(),
            },
            delete_wrapup: DeleteWrapUpDeps {
                wrapup_repo: repos.wrapup_repo.clone(),
            },
            generate: GenerateWrapUpDeps {
                wrapup_repo: repos.wrapup_repo.clone(),
            },
            get_wrapup: GetWrapUpDeps {
                wrapup_repo: repos.wrapup_repo.clone(),
            },
            list_wrapups: ListWrapUpsDeps {
                wrapup_repo: repos.wrapup_repo.clone(),
            },
        },
    }
}

/// The worker binary's assembly point: the five deps structs that have no consumer
/// the server binary can ever reach, built straight from `DatabaseOutput` — the
/// worker cannot honestly construct a `Repositories` (it lacks the six
/// federation-sourced ports) but every field this function reads is on
/// `DatabaseOutput` — plus `application::WorkerServices`, the strict subset of
/// ports the worker actually constructs.
pub fn build_worker_deps(
    db: &DatabaseOutput,
    services: &application::WorkerServices,
) -> WorkerDeps {
    WorkerDeps {
        enrich_movie: EnrichMovieDeps {
            movie_query: db.movie_query.clone(),
            movie_profile: db.movie_profile.clone(),
            person_command: db.person_command.clone(),
            search_command: db.search_command.clone(),
        },
        reindex_search: ReindexSearchDeps {
            movie_query: db.movie_query.clone(),
            movie_profile: db.movie_profile.clone(),
            search_command: db.search_command.clone(),
            person_command: db.person_command.clone(),
            person_query: db.person_query.clone(),
        },
        merge_duplicates: MergeDuplicatesDeps {
            movie_query: db.movie_query.clone(),
            deduplicator: db.deduplicator.clone(),
            object_storage: services.object_storage.clone(),
        },
        enrich_person: EnrichPersonDeps {
            person_query: db.person_query.clone(),
            person_enrichment: services.person_enrichment.clone(),
            person_command: db.person_command.clone(),
        },
        handle_requested: HandleWrapUpRequestedDeps {
            wrapup_repo: db.wrapup_repo.clone(),
            wrapup_stats: db.wrapup_stats.clone(),
        },
    }
}
