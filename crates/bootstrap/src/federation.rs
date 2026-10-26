use std::sync::Arc;

use domain::ports::{
    BlockQuery, EventPublisher, FederatedProfileQuery, FederationAdminQuery, FollowGraphQuery,
    RemoteWatchlistRepository, SocialCommand,
};
use domain::value_objects::InstanceIdentity;

use crate::database::DatabaseOutput;

pub struct ServerSocial {
    pub ap_router: axum::Router,
    #[cfg(feature = "federation")]
    pub ap_document: Arc<dyn domain::ports::ApDocumentPort>,
    #[cfg(feature = "federation")]
    pub ap_blocklist: Arc<dyn domain::ports::InstanceBlocklistPort>,
    pub social_command: Arc<dyn SocialCommand>,
    pub follow_graph: Arc<dyn FollowGraphQuery>,
    pub block_query: Arc<dyn BlockQuery>,
    pub remote_watchlist: Arc<dyn RemoteWatchlistRepository>,
    pub federation_admin: Arc<dyn FederationAdminQuery>,
    pub federated_profile: Option<Arc<dyn FederatedProfileQuery>>,
}

#[cfg(not(feature = "federation"))]
pub async fn wire_server_social(
    db: &DatabaseOutput,
    instance: &InstanceIdentity,
    _config: &config::AppConfig,
    _event_publisher: &Arc<dyn EventPublisher>,
) -> anyhow::Result<ServerSocial> {
    let local = Arc::new(application::social::local_service::LocalSocialService::new(
        Arc::clone(&db.user),
        Arc::clone(&db.follow_command),
        Arc::clone(&db.follow_query),
        instance.clone(),
    ));
    Ok(ServerSocial {
        ap_router: axum::Router::new(),
        social_command: Arc::clone(&local) as Arc<dyn SocialCommand>,
        follow_graph: Arc::clone(&local) as Arc<dyn FollowGraphQuery>,
        block_query: local as Arc<dyn BlockQuery>,
        remote_watchlist: Arc::new(domain::ports::noop::NoopRemoteWatchlistRepository),
        // The empty list this noop returns is an invariant, not an approximation:
        // federation-off, `LocalSocialService::follow_resolved` hard-errors on any
        // non-Local target and block/unblock always error, so no remote follow can
        // ever be persisted through this wiring. (Stale rows from an instance
        // previously built federation-ON are the one exception — display staleness
        // only; see ADR-0009.)
        federation_admin: Arc::new(domain::ports::noop::NoopFederationAdminQuery),
        federated_profile: None,
    })
}

#[cfg(feature = "federation")]
pub async fn wire_server_social(
    db: &DatabaseOutput,
    instance: &InstanceIdentity,
    config: &config::AppConfig,
    event_publisher: &Arc<dyn EventPublisher>,
) -> anyhow::Result<ServerSocial> {
    let fed_repos = sqlite_federation::wire(db.db_pool.clone(), instance.clone());

    let ap = activitypub::wire(activitypub::ActivityPubDeps {
        activity_repo: fed_repos.activity,
        follow_repo: fed_repos.follow,
        actor_repo: fed_repos.actor,
        blocklist_repo: fed_repos.blocklist,
        review_store: fed_repos.review_store,
        remote_watchlist_repo: fed_repos.remote_watchlist.clone(),
        remote_goal_repo: Arc::clone(&db.remote_goal),
        local_ap_content: Arc::clone(&db.ap_content),
        movie_repo: Arc::clone(&db.movie_query),
        review_repo: Arc::clone(&db.review),
        diary_repo: Arc::clone(&db.diary),
        goal_repo: Arc::clone(&db.goal_query),
        stats_repo: Arc::clone(&db.stats),
        user_repo: Arc::clone(&db.user),
        federation_settings: Arc::clone(&db.federation_settings),
        follow_command: Arc::clone(&fed_repos.follow_command),
        follow_query: Arc::clone(&fed_repos.follow_query),
        instance: instance.clone(),
        allow_registration: config.server.allow_registration,
        federation: config.federation.clone(),
        event_publisher: Arc::clone(event_publisher),
    })
    .await?;

    let local_social: Arc<dyn domain::ports::LocalSocial> =
        Arc::new(application::social::local_service::LocalSocialService::new(
            Arc::clone(&db.user),
            Arc::clone(&db.follow_command),
            Arc::clone(&db.follow_query),
            instance.clone(),
        ));

    let composite_social = Arc::new(activitypub::CompositeSocialAdapter::new(
        local_social,
        ap.service,
        Arc::clone(&db.user),
        instance.clone(),
        Arc::clone(event_publisher),
    ));

    Ok(ServerSocial {
        ap_router: ap.router,
        ap_document: ap.document,
        ap_blocklist: ap.blocklist,
        social_command: composite_social.clone() as Arc<dyn SocialCommand>,
        follow_graph: composite_social.clone() as Arc<dyn FollowGraphQuery>,
        block_query: composite_social as Arc<dyn BlockQuery>,
        remote_watchlist: fed_repos.remote_watchlist,
        federation_admin: fed_repos.admin_query,
        federated_profile: Some(sqlite::create_federated_profile_query(
            db.db_pool.clone(),
            instance.clone(),
        )),
    })
}

#[cfg(feature = "federation")]
pub async fn wire_worker_handlers(
    db: &DatabaseOutput,
    instance: &InstanceIdentity,
    config: &config::AppConfig,
    event_publisher: &Arc<dyn EventPublisher>,
) -> anyhow::Result<(
    Vec<Arc<dyn domain::ports::EventHandler>>,
    Arc<dyn domain::ports::FederationDeletionNotifier>,
)> {
    let fed_repos = sqlite_federation::wire(db.db_pool.clone(), instance.clone());

    let ap_wire = activitypub::wire(activitypub::ActivityPubDeps {
        activity_repo: fed_repos.activity,
        follow_repo: fed_repos.follow,
        actor_repo: fed_repos.actor,
        blocklist_repo: fed_repos.blocklist,
        review_store: fed_repos.review_store,
        remote_watchlist_repo: fed_repos.remote_watchlist,
        remote_goal_repo: Arc::clone(&db.remote_goal),
        local_ap_content: Arc::clone(&db.ap_content),
        movie_repo: Arc::clone(&db.movie_query),
        review_repo: Arc::clone(&db.review),
        diary_repo: Arc::clone(&db.diary),
        goal_repo: Arc::clone(&db.goal_query),
        stats_repo: Arc::clone(&db.stats),
        user_repo: Arc::clone(&db.user),
        follow_command: fed_repos.follow_command,
        follow_query: fed_repos.follow_query,
        instance: instance.clone(),
        allow_registration: config.server.allow_registration,
        federation: config.federation.clone(),
        event_publisher: Arc::clone(event_publisher),
        federation_settings: Arc::clone(&db.federation_settings),
    })
    .await?;

    tracing::info!("federation event handler registered");
    Ok((
        vec![
            ap_wire.event_handler,
            Arc::new(crate::follow_backfill_handler::FollowBackfillHandler {
                backfill: ap_wire.backfill,
            }),
        ],
        ap_wire.deletion_notifier,
    ))
}
