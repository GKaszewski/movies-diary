use domain::events::DomainEvent;

pub fn event_to_subject(prefix: &str, event: &DomainEvent) -> String {
    let suffix = match event {
        DomainEvent::ReviewLogged { .. }    => "review.logged",
        DomainEvent::ReviewUpdated { .. }   => "review.updated",
        DomainEvent::MovieDiscovered { .. } => "movie.discovered",
        DomainEvent::MovieDeleted { .. }    => "movie.deleted",
    };
    format!("{prefix}.{suffix}")
}

pub fn consumer_subject_filter(prefix: &str) -> String {
    format!("{prefix}.>")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;
    use domain::value_objects::{ExternalMetadataId, MovieId, Rating, ReviewId, UserId};
    use uuid::Uuid;

    fn dt() -> NaiveDateTime {
        chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap().naive_utc()
    }

    #[test]
    fn review_logged_subject() {
        let event = DomainEvent::ReviewLogged {
            review_id: ReviewId::from_uuid(Uuid::new_v4()),
            movie_id: MovieId::from_uuid(Uuid::new_v4()),
            user_id: UserId::from_uuid(Uuid::new_v4()),
            rating: Rating::new(3).unwrap(),
            watched_at: dt(),
        };
        assert_eq!(
            event_to_subject("movies-diary.events", &event),
            "movies-diary.events.review.logged"
        );
    }

    #[test]
    fn review_updated_subject() {
        let event = DomainEvent::ReviewUpdated {
            review_id: ReviewId::from_uuid(Uuid::new_v4()),
            movie_id: MovieId::from_uuid(Uuid::new_v4()),
            user_id: UserId::from_uuid(Uuid::new_v4()),
            rating: Rating::new(5).unwrap(),
            watched_at: dt(),
        };
        assert_eq!(
            event_to_subject("movies-diary.events", &event),
            "movies-diary.events.review.updated"
        );
    }

    #[test]
    fn movie_discovered_subject() {
        let event = DomainEvent::MovieDiscovered {
            movie_id: MovieId::from_uuid(Uuid::new_v4()),
            external_metadata_id: ExternalMetadataId::new("tt0000001".into()).unwrap(),
        };
        assert_eq!(
            event_to_subject("movies-diary.events", &event),
            "movies-diary.events.movie.discovered"
        );
    }

    #[test]
    fn consumer_subject_filter_appends_wildcard() {
        assert_eq!(
            consumer_subject_filter("movies-diary.events"),
            "movies-diary.events.>"
        );
    }
}
