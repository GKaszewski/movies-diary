use domain::events::DomainEvent;

pub fn event_to_subject(prefix: &str, event: &DomainEvent) -> String {
    let suffix = match event {
        DomainEvent::ReviewLogged { .. } => "review.logged",
        DomainEvent::ReviewUpdated { .. } => "review.updated",
        DomainEvent::ReviewDeleted { .. } => "review.deleted",
        DomainEvent::MovieDiscovered { .. } => "movie.discovered",
        DomainEvent::MovieDeleted { .. } => "movie.deleted",
        DomainEvent::UserUpdated { .. } => "user.updated",
        DomainEvent::MovieEnrichmentRequested { .. } => "movie.enrichment.requested",
        DomainEvent::ImageStored { .. } => "image.stored",
        DomainEvent::WatchlistEntryAdded { .. } => "watchlist.entry.added",
        DomainEvent::WatchlistEntryRemoved { .. } => "watchlist.entry.removed",
        DomainEvent::FollowRequested { .. } => "follow.requested",
        DomainEvent::FollowAccepted { .. } => "follow.accepted",
        DomainEvent::FollowRejected { .. } => "follow.rejected",
        DomainEvent::Unfollowed { .. } => "follow.unfollowed",
        DomainEvent::FollowerRemoved { .. } => "follower.removed",
        DomainEvent::ActorBlocked { .. } => "actor.blocked",
        DomainEvent::ActorUnblocked { .. } => "actor.unblocked",
        DomainEvent::BackfillFollower { .. } => "backfill.follower",
        DomainEvent::FederationDeliveryRequested { .. } => "federation.delivery.requested",
        DomainEvent::WatchEventIngested { .. } => "watch.event.ingested",
        DomainEvent::WrapUpRequested { .. } => "wrapup.requested",
        DomainEvent::WrapUpCompleted { .. } => "wrapup.completed",
        DomainEvent::SearchReindexRequested => "search.reindex.requested",
        DomainEvent::PosterSynced { .. } => "poster.synced",
        DomainEvent::GoalCreated { .. } => "goal.created",
        DomainEvent::GoalUpdated { .. } => "goal.updated",
        DomainEvent::GoalDeleted { .. } => "goal.deleted",
        DomainEvent::PersonEnrichmentRequested { .. } => "person.enrichment.requested",
        DomainEvent::UserDeleted { .. } => "user.deleted",
        DomainEvent::UserAccountMoved { .. } => "user.account.moved",
    };
    format!("{prefix}.{suffix}")
}

pub fn consumer_subject_filter(prefix: &str) -> String {
    format!("{prefix}.>")
}

#[cfg(test)]
#[path = "tests/subject.rs"]
mod tests;
