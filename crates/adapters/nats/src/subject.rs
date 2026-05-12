use domain::events::DomainEvent;

pub fn event_to_subject(prefix: &str, event: &DomainEvent) -> String {
    let suffix = match event {
        DomainEvent::ReviewLogged { .. }    => "review.logged",
        DomainEvent::ReviewUpdated { .. }   => "review.updated",
        DomainEvent::ReviewDeleted { .. }   => "review.deleted",
        DomainEvent::MovieDiscovered { .. } => "movie.discovered",
        DomainEvent::MovieDeleted { .. }              => "movie.deleted",
        DomainEvent::UserUpdated { .. }               => "user.updated",
        DomainEvent::MovieEnrichmentRequested { .. }  => "movie.enrichment.requested",
        DomainEvent::ImageStored { .. }              => "image.stored",
        DomainEvent::WatchlistEntryAdded { .. } | DomainEvent::WatchlistEntryRemoved { .. } => {
            unreachable!("watchlist events are not published to NATS")
        }
        DomainEvent::FollowAccepted { .. } => "follow.accepted",
    };
    format!("{prefix}.{suffix}")
}

pub fn consumer_subject_filter(prefix: &str) -> String {
    format!("{prefix}.>")
}

#[cfg(test)]
#[path = "tests/subject.rs"]
mod tests;
