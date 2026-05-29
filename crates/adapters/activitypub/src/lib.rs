pub mod composite_handler;
pub mod event_handler;
pub mod federation_event_bridge;
pub mod objects;
pub mod port;
pub mod remote_review_repository;
pub mod review_handler;
pub(crate) mod urls;
pub mod user_adapter;
pub mod watchlist_handler;

// Re-export the generic base types that callers need
pub use k_ap::{
    ActivityPubService, ActivityRepository, ActorRepository, ApContentReader, ApFederationConfig,
    ApObjectHandler, ApUser, ApUserRepository, BlocklistRepository, FederationData,
    FollowRepository, Follower, FollowerStatus, FollowingStatus, RemoteActor,
};

pub use event_handler::ActivityPubEventHandler;
pub use port::{ActivityPubPort, NoopActivityPubService};
pub use remote_review_repository::RemoteReviewRepository;
pub use review_handler::ReviewObjectHandler;
pub use user_adapter::DomainUserRepoAdapter;

pub struct ActivityPubWire {
    pub service: std::sync::Arc<dyn ActivityPubPort>,
    pub router: axum::Router,
    pub event_handler: std::sync::Arc<dyn domain::ports::EventHandler>,
}

pub async fn wire(
    activity_repo: std::sync::Arc<dyn ActivityRepository>,
    follow_repo: std::sync::Arc<dyn FollowRepository>,
    actor_repo: std::sync::Arc<dyn ActorRepository>,
    blocklist_repo: std::sync::Arc<dyn BlocklistRepository>,
    review_store: std::sync::Arc<dyn RemoteReviewRepository>,
    remote_watchlist_repo: std::sync::Arc<dyn domain::ports::RemoteWatchlistRepository>,
    local_ap_content: std::sync::Arc<dyn domain::ports::LocalApContentQuery>,
    user_repo: std::sync::Arc<dyn domain::ports::UserRepository>,
    base_url: String,
    allow_registration: bool,
    event_publisher: std::sync::Arc<dyn domain::ports::EventPublisher>,
) -> anyhow::Result<ActivityPubWire> {
    let review_handler = std::sync::Arc::new(ReviewObjectHandler {
        content_query: std::sync::Arc::clone(&local_ap_content),
        review_store,
        base_url: base_url.clone(),
    });
    let watchlist_handler = std::sync::Arc::new(watchlist_handler::WatchlistObjectHandler {
        remote_watchlist_repo,
        content_query: std::sync::Arc::clone(&local_ap_content),
        base_url: base_url.clone(),
    });
    let composite = std::sync::Arc::new(composite_handler::CompositeObjectHandler {
        review: review_handler,
        watchlist: watchlist_handler,
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
        base_url,
    )) as std::sync::Arc<dyn domain::ports::EventHandler>;

    Ok(ActivityPubWire {
        service: concrete as std::sync::Arc<dyn ActivityPubPort>,
        router,
        event_handler,
    })
}
