pub mod event_handler;
pub mod objects;
pub mod port;
pub mod remote_review_repository;
pub mod review_handler;
pub(crate) mod urls;
pub mod user_adapter;

// Re-export the generic base types that callers need
pub use activitypub_base::{
    ActivityPubService, ApFederationConfig, ApObjectHandler, ApUser, ApUserRepository,
    FederationData, FederationRepository, Follower, FollowerStatus, FollowingStatus, RemoteActor,
};

pub use event_handler::ActivityPubEventHandler;
pub use port::{ActivityPubPort, NoopActivityPubService};
pub use remote_review_repository::RemoteReviewRepository;
pub use review_handler::ReviewObjectHandler;
pub use user_adapter::DomainUserRepoAdapter;

pub struct ActivityPubWire {
    pub service:       std::sync::Arc<dyn ActivityPubPort>,
    pub router:        axum::Router,
    pub event_handler: std::sync::Arc<dyn domain::ports::EventHandler>,
}

pub async fn wire(
    federation_repo:    std::sync::Arc<dyn FederationRepository>,
    review_store:       std::sync::Arc<dyn RemoteReviewRepository>,
    user_repo:          std::sync::Arc<dyn domain::ports::UserRepository>,
    movie_repo:         std::sync::Arc<dyn domain::ports::MovieRepository>,
    review_repo:        std::sync::Arc<dyn domain::ports::ReviewRepository>,
    diary_repo:         std::sync::Arc<dyn domain::ports::DiaryRepository>,
    base_url:           String,
    allow_registration: bool,
) -> anyhow::Result<ActivityPubWire> {
    let concrete = std::sync::Arc::new(
        ActivityPubService::new(
            federation_repo,
            std::sync::Arc::new(DomainUserRepoAdapter::new(user_repo, base_url.clone())),
            std::sync::Arc::new(ReviewObjectHandler {
                movie_repository: std::sync::Arc::clone(&movie_repo),
                diary_repository: diary_repo,
                review_store,
                base_url: base_url.clone(),
            }),
            base_url.clone(),
            allow_registration,
            "movies-diary".to_string(),
            cfg!(debug_assertions),
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
