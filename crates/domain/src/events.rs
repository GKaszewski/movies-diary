use chrono::NaiveDateTime;

use crate::value_objects::{ExternalMetadataId, MovieId, Rating, ReviewId, UserId};

#[derive(Clone, Debug)]
pub enum DomainEvent {
    ReviewLogged {
        review_id: ReviewId,
        movie_id: MovieId,
        user_id: UserId,
        rating: Rating,
        watched_at: NaiveDateTime,
    },
    MovieDiscovered {
        movie_id: MovieId,
        external_metadata_id: ExternalMetadataId,
    },
}
