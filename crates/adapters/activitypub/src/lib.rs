pub mod composite_handler;
pub mod event_handler;
pub mod objects;
pub mod port;
pub mod remote_review_repository;
pub mod review_handler;
pub(crate) mod urls;
pub mod user_adapter;
pub mod watchlist_handler;

// Re-export the generic base types that callers need
pub use k_ap::{
    ActivityPubService, ApFederationConfig, ApObjectHandler, ApUser, ApUserRepository,
    FederationData, FederationRepository, Follower, FollowerStatus, FollowingStatus, RemoteActor,
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
    federation_repo: std::sync::Arc<dyn FederationRepository>,
    review_store: std::sync::Arc<dyn RemoteReviewRepository>,
    remote_watchlist_repo: std::sync::Arc<dyn domain::ports::RemoteWatchlistRepository>,
    user_repo: std::sync::Arc<dyn domain::ports::UserRepository>,
    movie_repo: std::sync::Arc<dyn domain::ports::MovieRepository>,
    review_repo: std::sync::Arc<dyn domain::ports::ReviewRepository>,
    diary_repo: std::sync::Arc<dyn domain::ports::DiaryRepository>,
    base_url: String,
    allow_registration: bool,
    event_publisher: std::sync::Arc<dyn domain::ports::EventPublisher>,
) -> anyhow::Result<ActivityPubWire> {
    let review_handler = std::sync::Arc::new(ReviewObjectHandler {
        movie_repository: std::sync::Arc::clone(&movie_repo),
        diary_repository: std::sync::Arc::clone(&diary_repo),
        review_store,
        base_url: base_url.clone(),
    });
    let watchlist_handler = std::sync::Arc::new(watchlist_handler::WatchlistObjectHandler {
        remote_watchlist_repo,
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

    let concrete = std::sync::Arc::new(
        ActivityPubService::new(
            federation_repo,
            std::sync::Arc::new(DomainUserRepoAdapter::new(user_repo, base_url.clone())),
            composite,
            base_url.clone(),
            allow_registration,
            "movies-diary".to_string(),
            federation_debug,
            Some(event_publisher),
        )
        .await?,
    );

    let router = concrete.router();
    let event_handler = std::sync::Arc::new(ActivityPubEventHandler::new(
        std::sync::Arc::clone(&concrete),
        movie_repo,
        review_repo,
        base_url,
    )) as std::sync::Arc<dyn domain::ports::EventHandler>;

    Ok(ActivityPubWire {
        service: concrete as std::sync::Arc<dyn ActivityPubPort>,
        router,
        event_handler,
    })
}
