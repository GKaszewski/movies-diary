use chrono::NaiveDateTime;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    value_objects::{ExternalMetadataId, MovieId, Rating, ReviewId, UserId},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum DbEventPayload {
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
}

impl DbEventPayload {
    pub fn event_type(&self) -> &'static str {
        match self {
            DbEventPayload::ReviewLogged { .. } => "ReviewLogged",
            DbEventPayload::ReviewUpdated { .. } => "ReviewUpdated",
            DbEventPayload::MovieDiscovered { .. } => "MovieDiscovered",
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

impl From<&DomainEvent> for DbEventPayload {
    fn from(event: &DomainEvent) -> Self {
        match event {
            DomainEvent::ReviewLogged { review_id, movie_id, user_id, rating, watched_at } => {
                DbEventPayload::ReviewLogged {
                    review_id: review_id.value().to_string(),
                    movie_id: movie_id.value().to_string(),
                    user_id: user_id.value().to_string(),
                    rating: rating.value(),
                    watched_at: watched_at.and_utc().timestamp(),
                }
            }
            DomainEvent::ReviewUpdated { review_id, movie_id, user_id, rating, watched_at } => {
                DbEventPayload::ReviewUpdated {
                    review_id: review_id.value().to_string(),
                    movie_id: movie_id.value().to_string(),
                    user_id: user_id.value().to_string(),
                    rating: rating.value(),
                    watched_at: watched_at.and_utc().timestamp(),
                }
            }
            DomainEvent::MovieDiscovered { movie_id, external_metadata_id } => {
                DbEventPayload::MovieDiscovered {
                    movie_id: movie_id.value().to_string(),
                    external_metadata_id: external_metadata_id.value().to_owned(),
                }
            }
        }
    }
}

impl TryFrom<DbEventPayload> for DomainEvent {
    type Error = DomainError;
    fn try_from(payload: DbEventPayload) -> Result<Self, DomainError> {
        match payload {
            DbEventPayload::ReviewLogged { review_id, movie_id, user_id, rating, watched_at } => {
                Ok(DomainEvent::ReviewLogged {
                    review_id: ReviewId::from_uuid(parse_uuid(&review_id, "review_id")?),
                    movie_id: MovieId::from_uuid(parse_uuid(&movie_id, "movie_id")?),
                    user_id: UserId::from_uuid(parse_uuid(&user_id, "user_id")?),
                    rating: Rating::new(rating)?,
                    watched_at: parse_ts(watched_at)?,
                })
            }
            DbEventPayload::ReviewUpdated { review_id, movie_id, user_id, rating, watched_at } => {
                Ok(DomainEvent::ReviewUpdated {
                    review_id: ReviewId::from_uuid(parse_uuid(&review_id, "review_id")?),
                    movie_id: MovieId::from_uuid(parse_uuid(&movie_id, "movie_id")?),
                    user_id: UserId::from_uuid(parse_uuid(&user_id, "user_id")?),
                    rating: Rating::new(rating)?,
                    watched_at: parse_ts(watched_at)?,
                })
            }
            DbEventPayload::MovieDiscovered { movie_id, external_metadata_id } => {
                Ok(DomainEvent::MovieDiscovered {
                    movie_id: MovieId::from_uuid(parse_uuid(&movie_id, "movie_id")?),
                    external_metadata_id: ExternalMetadataId::new(external_metadata_id)?,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_dt() -> NaiveDateTime {
        chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap().naive_utc()
    }

    fn review_logged() -> DomainEvent {
        DomainEvent::ReviewLogged {
            review_id: ReviewId::from_uuid(Uuid::new_v4()),
            movie_id: MovieId::from_uuid(Uuid::new_v4()),
            user_id: UserId::from_uuid(Uuid::new_v4()),
            rating: Rating::new(4).unwrap(),
            watched_at: fixed_dt(),
        }
    }

    fn review_updated() -> DomainEvent {
        DomainEvent::ReviewUpdated {
            review_id: ReviewId::from_uuid(Uuid::new_v4()),
            movie_id: MovieId::from_uuid(Uuid::new_v4()),
            user_id: UserId::from_uuid(Uuid::new_v4()),
            rating: Rating::new(3).unwrap(),
            watched_at: fixed_dt(),
        }
    }

    fn movie_discovered() -> DomainEvent {
        DomainEvent::MovieDiscovered {
            movie_id: MovieId::from_uuid(Uuid::new_v4()),
            external_metadata_id: ExternalMetadataId::new("tt1234567".into()).unwrap(),
        }
    }

    fn round_trip(event: DomainEvent) {
        let payload = DbEventPayload::from(&event);
        let json = serde_json::to_string(&payload).expect("serialize");
        let back: DbEventPayload = serde_json::from_str(&json).expect("deserialize");
        let recovered = DomainEvent::try_from(back).expect("try_from");
        assert_eq!(DbEventPayload::from(&event), DbEventPayload::from(&recovered));
    }

    #[test]
    fn round_trip_review_logged() {
        round_trip(review_logged());
    }

    #[test]
    fn round_trip_review_updated() {
        round_trip(review_updated());
    }

    #[test]
    fn round_trip_movie_discovered() {
        round_trip(movie_discovered());
    }

    #[test]
    fn serialized_format_is_tagged() {
        let payload = DbEventPayload::from(&movie_discovered());
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains(r#""type":"MovieDiscovered""#));
        assert!(json.contains(r#""data":"#));
    }

    #[test]
    fn event_type_strings() {
        assert_eq!(DbEventPayload::from(&review_logged()).event_type(), "ReviewLogged");
        assert_eq!(DbEventPayload::from(&review_updated()).event_type(), "ReviewUpdated");
        assert_eq!(DbEventPayload::from(&movie_discovered()).event_type(), "MovieDiscovered");
    }
}
