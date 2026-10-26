use std::sync::Arc;

use domain::ports::{EventPublisher, MediaServerParser, ObjectStorage, PersonEnrichmentClient};

use crate::auth::deps::{
    AuthenticateDeps, LoginDeps, LogoutDeps, RefreshDeps, RegisterAndLoginDeps, RegisterDeps,
};
use crate::diary::deps::{
    DeleteReviewDeps, EditReviewDeps, ExportDiaryDeps, GetActivityFeedDeps, GetDiaryDeps,
    GetMovieSocialPageDeps, GetReviewHistoryDeps, GetUserFeedDeps,
};
use crate::goals::deps::{GoalCommandDeps, GoalQueryDeps};
use crate::images::deps::GetImageDeps;
use crate::import::deps::{
    ApplyMappingDeps, ApplyProfileAndMapDeps, ApplyProfileDeps, CreateSessionDeps,
    DeleteImportProfileDeps, ExecuteImportDeps, GetMappingStageDeps, GetPreviewStageDeps,
    GetSessionStateDeps, ListImportProfilesDeps, SaveProfileDeps,
};
use crate::insights::deps::GetInsightsDeps;
use crate::integrations::deps::{
    ConfirmWatchEventsDeps, DismissWatchEventsDeps, GenerateWebhookTokenDeps, GetWatchQueueDeps,
    GetWebhookTokensDeps, IngestWatchEventDeps, RevokeWebhookTokenDeps,
};
use crate::movies::deps::{
    EnrichMovieDeps, GetMovieProfileDeps, GetMoviesDeps, ReindexSearchDeps, SyncPosterDeps,
};
use crate::movies::merge_duplicates::MergeDuplicatesDeps;
use crate::person::deps::{EnrichPersonDeps, GetPersonDeps};
use crate::search::deps::{RequestReindexDeps, SearchDeps};
use crate::social::deps::{SocialCommandDeps, SocialQueryDeps};
use crate::users::deps::{
    AuthorizeAdminDeps, GetCurrentProfileDeps, GetFederatedProfileDeps,
    GetFederatedProfileStatsDeps, GetLocalProfileDeps, GetPageViewerDeps, GetProfileSettingsDeps,
    GetSettingsDeps, GetUsersListDeps, RequestAccountDeletionDeps, ResolveUsernameDeps,
    UpdateProfileDeps, UpdateProfileFieldsDeps, UpdateSettingsDeps,
};
use crate::watchlist::deps::{
    GetWatchlistDeps, GetWatchlistForOwnerDeps, IsOnWatchlistDeps, RemoveFromWatchlistDeps,
    WatchlistAddDeps,
};
use crate::wrapup::deps::{
    DeleteWrapUpDeps, GenerateWrapUpDeps, GetReadyReportDeps, GetWrapUpDeps,
    HandleWrapUpRequestedDeps, ListWrapUpsDeps,
};

pub struct AuthGroup {
    pub login: LoginDeps,
    pub register: RegisterDeps,
    pub refresh: RefreshDeps,
    pub register_and_login: RegisterAndLoginDeps,
    pub logout: LogoutDeps,
    pub authenticate: AuthenticateDeps,
}

pub struct DiaryGroup {
    pub delete_review: DeleteReviewDeps,
    pub edit_review: EditReviewDeps,
    pub get_movie_social_page: GetMovieSocialPageDeps,
    pub get_activity_feed: GetActivityFeedDeps,
    pub get_user_feed: GetUserFeedDeps,
    pub get_diary: GetDiaryDeps,
    pub get_review_history: GetReviewHistoryDeps,
    pub export_diary: ExportDiaryDeps,
}

pub struct GoalsGroup {
    pub command: GoalCommandDeps,
    pub query: GoalQueryDeps,
}

pub struct ImagesGroup {
    pub get_image: GetImageDeps,
}

pub struct ImportGroup {
    pub create_session: CreateSessionDeps,
    pub apply_mapping: ApplyMappingDeps,
    pub apply_profile: ApplyProfileDeps,
    pub execute_import: ExecuteImportDeps,
    pub save_profile: SaveProfileDeps,
    pub get_mapping_stage: GetMappingStageDeps,
    pub get_preview_stage: GetPreviewStageDeps,
    pub get_session_state: GetSessionStateDeps,
    pub apply_profile_and_map: ApplyProfileAndMapDeps,
    pub delete_profile: DeleteImportProfileDeps,
    pub list_profiles: ListImportProfilesDeps,
}

pub struct IntegrationsGroup {
    pub ingest_watch_event: IngestWatchEventDeps,
    pub confirm_watch_events: ConfirmWatchEventsDeps,
    pub dismiss_watch_events: DismissWatchEventsDeps,
    pub generate_webhook_token: GenerateWebhookTokenDeps,
    pub get_watch_queue: GetWatchQueueDeps,
    pub get_webhook_tokens: GetWebhookTokensDeps,
    pub revoke_webhook_token: RevokeWebhookTokenDeps,
    /// Webhook payload parsers. Held on the group rather than inside
    /// `IngestWatchEventDeps` because `ingest::execute` takes the parser as an
    /// argument — the caller picks which one per route.
    pub jellyfin_parser: Arc<dyn MediaServerParser>,
    pub plex_parser: Arc<dyn MediaServerParser>,
}

pub struct InsightsGroup {
    pub get: GetInsightsDeps,
}

pub struct MoviesGroup {
    pub sync_poster: SyncPosterDeps,
    pub get_movie_profile: GetMovieProfileDeps,
    pub get_movies: GetMoviesDeps,
}

pub struct PersonGroup {
    pub get_person: GetPersonDeps,
}

pub struct SearchGroup {
    pub execute: SearchDeps,
    pub request_reindex: RequestReindexDeps,
}

pub struct SocialGroup {
    pub command: SocialCommandDeps,
    pub query: SocialQueryDeps,
}

pub struct UsersGroup {
    pub get_local_profile: GetLocalProfileDeps,
    pub get_federated_profile_stats: GetFederatedProfileStatsDeps,
    pub get_page_viewer: GetPageViewerDeps,
    pub resolve_username: ResolveUsernameDeps,
    pub get_profile_settings: GetProfileSettingsDeps,
    pub get_users_list: GetUsersListDeps,
    pub update_profile: UpdateProfileDeps,
    pub request_account_deletion: RequestAccountDeletionDeps,
    pub get_current_profile: GetCurrentProfileDeps,
    pub update_profile_fields: UpdateProfileFieldsDeps,
    pub get_settings: GetSettingsDeps,
    pub update_settings: UpdateSettingsDeps,
    pub authorize_admin: AuthorizeAdminDeps,
    pub get_federated_profile: GetFederatedProfileDeps,
}

pub struct WatchlistGroup {
    pub add: WatchlistAddDeps,
    pub get_watchlist_for_owner: GetWatchlistForOwnerDeps,
    pub get_watchlist: GetWatchlistDeps,
    pub is_on_watchlist: IsOnWatchlistDeps,
    pub remove_from_watchlist: RemoveFromWatchlistDeps,
}

pub struct WrapupGroup {
    pub get_ready_report: GetReadyReportDeps,
    pub delete_wrapup: DeleteWrapUpDeps,
    pub generate: GenerateWrapUpDeps,
    pub get_wrapup: GetWrapUpDeps,
    pub list_wrapups: ListWrapUpsDeps,
}

/// Every deps struct a handler can need, built once by `bootstrap`.
/// Use cases still receive only their own narrow struct — nothing takes `&Deps`.
///
/// `bootstrap::build_deps` is called only from `bootstrap::build_server`.
/// The worker-only groups this used to also carry now live in `WorkerDeps`, built by
/// `bootstrap::build_worker_deps` and consumed by `bootstrap::build_worker` —
/// the worker binary no longer wires anything itself. Nothing
/// in `Deps` below is worker-only anymore, except one field with no consumer anywhere
/// in the workspace — see its comment.
pub struct Deps {
    pub auth: AuthGroup,
    pub diary: DiaryGroup,
    pub goals: GoalsGroup,
    pub images: ImagesGroup,
    pub import: ImportGroup,
    pub insights: InsightsGroup,
    pub integrations: IntegrationsGroup,
    pub movies: MoviesGroup,
    pub person: PersonGroup,
    pub search: SearchGroup,
    pub social: SocialGroup,
    pub users: UsersGroup,
    pub watchlist: WatchlistGroup,
    pub wrapup: WrapupGroup,
}

/// Ports the worker binary can actually construct — a strict subset of
/// `Services`. The worker has no `auth`, `password_hasher`, `diary_exporter`,
/// `document_parser`, or `review_logger`; those ports have no worker-side use case,
/// so `WorkerServices` simply does not carry them (see ADR / task-2 brief for why
/// this is a separate struct rather than an `Option`-riddled `Services`).
pub struct WorkerServices {
    pub object_storage: Arc<dyn ObjectStorage>,
    pub event_publisher: Arc<dyn EventPublisher>,
    /// `Option` here mirrors `Services::person_enrichment` — genuine optional
    /// configuration, not a container-shape workaround.
    pub person_enrichment: Option<Arc<dyn PersonEnrichmentClient>>,
}

/// The deps structs the worker binary needs, built by `bootstrap::build_worker_deps`.
/// These are the five groups that moved out of `Deps` during worker unification —
/// they have no consumer the server binary can ever reach.
pub struct WorkerDeps {
    pub enrich_movie: EnrichMovieDeps,
    pub reindex_search: ReindexSearchDeps,
    pub merge_duplicates: MergeDuplicatesDeps,
    pub enrich_person: EnrichPersonDeps,
    pub handle_requested: HandleWrapUpRequestedDeps,
}
