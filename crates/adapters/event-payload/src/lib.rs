use chrono::NaiveDateTime;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    value_objects::{ExternalMetadataId, MovieId, PosterPath, Rating, ReviewId, UserId},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum EventPayload {
    ReviewLogged {
        review_id: String,
        movie_id: String,
        user_id: String,
        rating: u8,
        watched_at: i64,
    },
    ReviewUpdated {
        review_id: String,
        movie_id: String,
        user_id: String,
        rating: u8,
        watched_at: i64,
    },
    MovieDiscovered {
        movie_id: String,
        external_metadata_id: String,
    },
    MovieDeleted {
        movie_id: String,
        poster_path: Option<String>,
    },
    UserUpdated {
        user_id: String,
    },
    ReviewDeleted {
        review_id: String,
        user_id: String,
    },
    MovieEnrichmentRequested {
        movie_id: String,
        external_metadata_id: String,
    },
    ImageStored {
        key: String,
    },
}

impl EventPayload {
    pub fn event_type(&self) -> &'static str {
        match self {
            EventPayload::ReviewLogged { .. } => "ReviewLogged",
            EventPayload::ReviewUpdated { .. } => "ReviewUpdated",
            EventPayload::MovieDiscovered { .. } => "MovieDiscovered",
            EventPayload::MovieDeleted { .. } => "MovieDeleted",
            EventPayload::UserUpdated { .. } => "UserUpdated",
            EventPayload::ReviewDeleted { .. } => "ReviewDeleted",
            EventPayload::MovieEnrichmentRequested { .. } => "MovieEnrichmentRequested",
            EventPayload::ImageStored { .. } => "ImageStored",
        }
    }
}

fn parse_uuid(s: &str, field: &str) -> Result<Uuid, DomainError> {
    Uuid::parse_str(s)
        .map_err(|e| DomainError::InfrastructureError(format!("{field}: {e}")))
}

fn parse_ts(ts: i64) -> Result<NaiveDateTime, DomainError> {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|dt| dt.naive_utc())
        .ok_or_else(|| DomainError::InfrastructureError(format!("invalid timestamp: {ts}")))
}

impl From<&DomainEvent> for EventPayload {
    fn from(event: &DomainEvent) -> Self {
        match event {
            DomainEvent::ReviewLogged { review_id, movie_id, user_id, rating, watched_at } => {
                EventPayload::ReviewLogged {
                    review_id: review_id.value().to_string(),
                    movie_id: movie_id.value().to_string(),
                    user_id: user_id.value().to_string(),
                    rating: rating.value(),
                    watched_at: watched_at.and_utc().timestamp(),
                }
            }
            DomainEvent::ReviewUpdated { review_id, movie_id, user_id, rating, watched_at } => {
                EventPayload::ReviewUpdated {
                    review_id: review_id.value().to_string(),
                    movie_id: movie_id.value().to_string(),
                    user_id: user_id.value().to_string(),
                    rating: rating.value(),
                    watched_at: watched_at.and_utc().timestamp(),
                }
            }
            DomainEvent::MovieDiscovered { movie_id, external_metadata_id } => {
                EventPayload::MovieDiscovered {
                    movie_id: movie_id.value().to_string(),
                    external_metadata_id: external_metadata_id.value().to_owned(),
                }
            }
            DomainEvent::MovieDeleted { movie_id, poster_path } => EventPayload::MovieDeleted {
                movie_id: movie_id.value().to_string(),
                poster_path: poster_path.as_ref().map(|p| p.value().to_string()),
            },
            DomainEvent::UserUpdated { user_id } => EventPayload::UserUpdated {
                user_id: user_id.value().to_string(),
            },
            DomainEvent::ReviewDeleted { review_id, user_id } => EventPayload::ReviewDeleted {
                review_id: review_id.value().to_string(),
                user_id: user_id.value().to_string(),
            },
            DomainEvent::MovieEnrichmentRequested { movie_id, external_metadata_id } => {
                EventPayload::MovieEnrichmentRequested {
                    movie_id: movie_id.value().to_string(),
                    external_metadata_id: external_metadata_id.clone(),
                }
            }
            DomainEvent::ImageStored { key } => EventPayload::ImageStored { key: key.clone() },
        }
    }
}

impl TryFrom<EventPayload> for DomainEvent {
    type Error = DomainError;
    fn try_from(payload: EventPayload) -> Result<Self, DomainError> {
        match payload {
            EventPayload::ReviewLogged { review_id, movie_id, user_id, rating, watched_at } => {
                Ok(DomainEvent::ReviewLogged {
                    review_id: ReviewId::from_uuid(parse_uuid(&review_id, "review_id")?),
                    movie_id: MovieId::from_uuid(parse_uuid(&movie_id, "movie_id")?),
                    user_id: UserId::from_uuid(parse_uuid(&user_id, "user_id")?),
                    rating: Rating::new(rating)?,
                    watched_at: parse_ts(watched_at)?,
                })
            }
            EventPayload::ReviewUpdated { review_id, movie_id, user_id, rating, watched_at } => {
                Ok(DomainEvent::ReviewUpdated {
                    review_id: ReviewId::from_uuid(parse_uuid(&review_id, "review_id")?),
                    movie_id: MovieId::from_uuid(parse_uuid(&movie_id, "movie_id")?),
                    user_id: UserId::from_uuid(parse_uuid(&user_id, "user_id")?),
                    rating: Rating::new(rating)?,
                    watched_at: parse_ts(watched_at)?,
                })
            }
            EventPayload::MovieDiscovered { movie_id, external_metadata_id } => {
                Ok(DomainEvent::MovieDiscovered {
                    movie_id: MovieId::from_uuid(parse_uuid(&movie_id, "movie_id")?),
                    external_metadata_id: ExternalMetadataId::new(external_metadata_id)?,
                })
            }
            EventPayload::MovieDeleted { movie_id, poster_path } => {
                let movie_id = MovieId::from_uuid(parse_uuid(&movie_id, "movie_id")?);
                let poster_path = poster_path
                    .map(|p| PosterPath::new(p))
                    .transpose()
                    .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
                Ok(DomainEvent::MovieDeleted { movie_id, poster_path })
            }
            EventPayload::UserUpdated { user_id } => {
                Ok(DomainEvent::UserUpdated {
                    user_id: UserId::from_uuid(parse_uuid(&user_id, "user_id")?),
                })
            }
            EventPayload::ReviewDeleted { review_id, user_id } => {
                Ok(DomainEvent::ReviewDeleted {
                    review_id: ReviewId::from_uuid(parse_uuid(&review_id, "review_id")?),
                    user_id: UserId::from_uuid(parse_uuid(&user_id, "user_id")?),
                })
            }
            EventPayload::MovieEnrichmentRequested { movie_id, external_metadata_id } => {
                Ok(DomainEvent::MovieEnrichmentRequested {
                    movie_id: MovieId::from_uuid(parse_uuid(&movie_id, "movie_id")?),
                    external_metadata_id,
                })
            }
            EventPayload::ImageStored { key } => {
                Ok(DomainEvent::ImageStored { key })
            }
        }
    }
}

#[cfg(test)]
#[path = "tests/lib.rs"]
mod tests;
