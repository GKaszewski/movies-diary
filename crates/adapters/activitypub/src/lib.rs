pub mod composite_handler;
pub mod event_handler;
pub mod federation_event_bridge;
pub mod goal_handler;
pub mod objects;
pub mod port;
pub mod remote_review_repository;
pub mod review_handler;
pub(crate) mod urls;
pub mod user_adapter;
pub mod watchlist_handler;

pub const INSTANCE_ACTOR_ID: uuid::Uuid =
    uuid::Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0x40, 0, 0x80, 0, 0, 0, 0, 0, 0, 0]);

// Re-export the generic base types that callers need
pub use k_ap::{
    ActivityPubService, ActivityRepository, ActorRepository, ApContentReader, ApFederationConfig,
    ApObjectHandler, ApUser, ApUserRepository, BlocklistRepository, FederationData,
    FollowRepository, Follower, FollowerStatus, FollowingStatus, RemoteActor,
};

pub use event_handler::ActivityPubEventHandler;
pub use port::{ActivityPubPort, NoopActivityPubService};
pub use remote_review_repository::{RemoteReviewRepository, RemoteReviewUpdate};
pub use review_handler::ReviewObjectHandler;
pub use user_adapter::DomainUserRepoAdapter;

pub type FederationRepos = (
    std::sync::Arc<dyn ActivityRepository>,
    std::sync::Arc<dyn FollowRepository>,
    std::sync::Arc<dyn ActorRepository>,
    std::sync::Arc<dyn BlocklistRepository>,
    std::sync::Arc<dyn domain::ports::SocialQueryPort>,
    std::sync::Arc<dyn RemoteReviewRepository>,
    std::sync::Arc<dyn domain::ports::RemoteWatchlistRepository>,
);

pub struct ActivityPubWire {
    pub service: std::sync::Arc<dyn ActivityPubPort>,
    pub router: axum::Router,
    pub event_handler: std::sync::Arc<dyn domain::ports::EventHandler>,
}

pub struct ActivityPubDeps {
    pub activity_repo: std::sync::Arc<dyn ActivityRepository>,
    pub follow_repo: std::sync::Arc<dyn FollowRepository>,
    pub actor_repo: std::sync::Arc<dyn ActorRepository>,
    pub blocklist_repo: std::sync::Arc<dyn BlocklistRepository>,
    pub review_store: std::sync::Arc<dyn RemoteReviewRepository>,
    pub remote_watchlist_repo: std::sync::Arc<dyn domain::ports::RemoteWatchlistRepository>,
    pub remote_goal_repo: std::sync::Arc<dyn domain::ports::RemoteGoalRepository>,
    pub local_ap_content: std::sync::Arc<dyn domain::ports::LocalApContentQuery>,
    pub movie_repo: std::sync::Arc<dyn domain::ports::MovieQuery>,
    pub review_repo: std::sync::Arc<dyn domain::ports::ReviewRepository>,
    pub diary_repo: std::sync::Arc<dyn domain::ports::DiaryQuery>,
    pub goal_repo: std::sync::Arc<dyn domain::ports::GoalQuery>,
    pub stats_repo: std::sync::Arc<dyn domain::ports::StatsRepository>,
    pub user_repo: std::sync::Arc<dyn domain::ports::UserRepository>,
    pub federation_settings: std::sync::Arc<dyn domain::ports::UserFederationSettingsQuery>,
    pub base_url: String,
    pub allow_registration: bool,
    pub event_publisher: std::sync::Arc<dyn domain::ports::EventPublisher>,
}

pub async fn wire(deps: ActivityPubDeps) -> anyhow::Result<ActivityPubWire> {
    let ActivityPubDeps {
        activity_repo,
        follow_repo,
        actor_repo,
        blocklist_repo,
        review_store,
        remote_watchlist_repo,
        remote_goal_repo,
        local_ap_content,
        movie_repo,
        review_repo,
        diary_repo,
        goal_repo,
        stats_repo,
        user_repo,
        federation_settings,
        base_url,
        allow_registration,
        event_publisher,
    } = deps;
    let review_handler = std::sync::Arc::new(ReviewObjectHandler {
        content_query: std::sync::Arc::clone(&local_ap_content),
        movie_repo: std::sync::Arc::clone(&movie_repo),
        diary_repo,
        review_store,
        event_publisher: std::sync::Arc::clone(&event_publisher),
        base_url: base_url.clone(),
    });
    let watchlist_handler = std::sync::Arc::new(watchlist_handler::WatchlistObjectHandler {
        remote_watchlist_repo,
        content_query: std::sync::Arc::clone(&local_ap_content),
        base_url: base_url.clone(),
    });
    let goal_handler = std::sync::Arc::new(goal_handler::GoalObjectHandler {
        remote_goal_repo,
        goal_repo: std::sync::Arc::clone(&goal_repo),
        base_url: base_url.clone(),
    });
    let composite = std::sync::Arc::new(composite_handler::CompositeObjectHandler {
        review: review_handler,
        watchlist: watchlist_handler,
        goal: goal_handler,
    });

    let federation_debug = std::env::var("FEDERATION_DEBUG")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    if federation_debug {
        tracing::warn!(
            "federation running in DEBUG mode — PermissiveVerifier active, \
             no URL/signature validation. Do NOT use in production."
        );
    }

    let fed_event_bridge = std::sync::Arc::new(
        federation_event_bridge::FederationEventBridge::new(event_publisher),
    );

    let concrete = std::sync::Arc::new(
        ActivityPubService::builder(base_url.clone())
            .activity_repo(activity_repo)
            .follow_repo(follow_repo)
            .actor_repo(actor_repo)
            .blocklist_repo(blocklist_repo)
            .user_repo(std::sync::Arc::new(DomainUserRepoAdapter::new(
                user_repo,
                base_url.clone(),
            )))
            .signed_fetch_actor_id(INSTANCE_ACTOR_ID)
            .content_reader(composite.clone() as std::sync::Arc<dyn ApContentReader>)
            .object_handler(composite as std::sync::Arc<dyn ApObjectHandler>)
            .event_publisher(fed_event_bridge)
            .allow_registration(allow_registration)
            .software_name("movies-diary")
            .debug(federation_debug)
            .build()
            .await?,
    );

    let router = concrete.router();
    let event_handler = std::sync::Arc::new(ActivityPubEventHandler::new(
        std::sync::Arc::clone(&concrete),
        local_ap_content,
        review_repo,
        movie_repo,
        goal_repo,
        stats_repo,
        federation_settings,
        base_url,
    )) as std::sync::Arc<dyn domain::ports::EventHandler>;

    Ok(ActivityPubWire {
        service: concrete as std::sync::Arc<dyn ActivityPubPort>,
        router,
        event_handler,
    })
}
