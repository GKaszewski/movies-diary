use chrono::NaiveDateTime;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{ExternalPersonId, PersonId},
    value_objects::{
        ExternalMetadataId, GoalId, MovieId, PosterPath, Rating, ReviewId, SocialIdentity, UserId,
        WrapUpId,
    },
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
    WatchlistEntryAdded {
        user_id: String,
        movie_id: String,
        movie_title: String,
        release_year: u16,
        external_metadata_id: Option<String>,
        added_at: i64,
    },
    WatchlistEntryRemoved {
        user_id: String,
        movie_id: String,
    },
    FollowRequested {
        follower_id: String,
        target_kind: String,
        target_id: String,
    },
    FollowAccepted {
        owner_id: String,
        requester_kind: String,
        requester_id: String,
    },
    Unfollowed {
        follower_id: String,
        target_kind: String,
        target_id: String,
    },
    FollowerRemoved {
        owner_id: String,
        follower_kind: String,
        follower_id: String,
    },
    ActorBlocked {
        blocker_id: String,
        target_kind: String,
        target_id: String,
    },
    ActorUnblocked {
        blocker_id: String,
        target_kind: String,
        target_id: String,
    },
    BackfillFollower {
        owner_user_id: String,
        follower_inbox_url: String,
    },
    FederationDeliveryRequested {
        inbox_url: String,
        activity_json: serde_json::Value,
        signing_actor_id: String,
    },
    WatchEventIngested {
        user_id: String,
        title: String,
        source: String,
    },
    WrapUpRequested {
        wrapup_id: String,
        user_id: Option<String>,
        start_date: String,
        end_date: String,
    },
    WrapUpCompleted {
        wrapup_id: String,
    },
    SearchReindexRequested,
    PosterSynced {
        movie_id: String,
    },
    GoalCreated {
        goal_id: String,
        user_id: String,
        year: u16,
        target_count: u32,
    },
    GoalUpdated {
        goal_id: String,
        user_id: String,
        year: u16,
        target_count: u32,
    },
    GoalDeleted {
        goal_id: String,
        user_id: String,
        year: u16,
    },
    PersonEnrichmentRequested {
        person_id: String,
        external_person_id: String,
    },
    UserDeleted {
        user_id: String,
    },
    UserAccountMoved {
        user_id: String,
        new_actor_url: String,
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
            EventPayload::WatchlistEntryAdded { .. } => "WatchlistEntryAdded",
            EventPayload::WatchlistEntryRemoved { .. } => "WatchlistEntryRemoved",
            EventPayload::FollowRequested { .. } => "FollowRequested",
            EventPayload::FollowAccepted { .. } => "FollowAccepted",
            EventPayload::Unfollowed { .. } => "Unfollowed",
            EventPayload::FollowerRemoved { .. } => "FollowerRemoved",
            EventPayload::ActorBlocked { .. } => "ActorBlocked",
            EventPayload::ActorUnblocked { .. } => "ActorUnblocked",
            EventPayload::BackfillFollower { .. } => "BackfillFollower",
            EventPayload::FederationDeliveryRequested { .. } => "FederationDeliveryRequested",
            EventPayload::WatchEventIngested { .. } => "WatchEventIngested",
            EventPayload::WrapUpRequested { .. } => "WrapUpRequested",
            EventPayload::WrapUpCompleted { .. } => "WrapUpCompleted",
            EventPayload::SearchReindexRequested => "SearchReindexRequested",
            EventPayload::PosterSynced { .. } => "PosterSynced",
            EventPayload::GoalCreated { .. } => "GoalCreated",
            EventPayload::GoalUpdated { .. } => "GoalUpdated",
            EventPayload::GoalDeleted { .. } => "GoalDeleted",
            EventPayload::PersonEnrichmentRequested { .. } => "PersonEnrichmentRequested",
            EventPayload::UserDeleted { .. } => "UserDeleted",
            EventPayload::UserAccountMoved { .. } => "UserAccountMoved",
        }
    }
}

fn parse_uuid(s: &str, field: &str) -> Result<Uuid, DomainError> {
    Uuid::parse_str(s).map_err(|e| DomainError::InfrastructureError(format!("{field}: {e}")))
}

fn identity_to_payload(id: &SocialIdentity) -> (String, String) {
    match id {
        SocialIdentity::Local(uid) => ("local".into(), uid.value().to_string()),
        SocialIdentity::Remote { actor_url } => ("remote".into(), actor_url.clone()),
    }
}

fn follow_target_to_payload(target: &domain::value_objects::FollowTarget) -> (String, String) {
    match target {
        domain::value_objects::FollowTarget::Identity(id) => identity_to_payload(id),
        domain::value_objects::FollowTarget::Handle(h) => ("handle".into(), h.clone()),
    }
}

fn payload_to_identity(kind: &str, id: String) -> Result<SocialIdentity, DomainError> {
    match kind {
        "local" => Ok(SocialIdentity::Local(UserId::from_uuid(parse_uuid(
            &id, "user_id",
        )?))),
        "remote" => Ok(SocialIdentity::Remote { actor_url: id }),
        other => Err(DomainError::InfrastructureError(format!(
            "unknown identity kind: {other}"
        ))),
    }
}

fn payload_to_follow_target(
    kind: &str,
    id: String,
) -> Result<domain::value_objects::FollowTarget, DomainError> {
    match kind {
        "handle" => Ok(domain::value_objects::FollowTarget::Handle(id)),
        other => Ok(domain::value_objects::FollowTarget::Identity(
            payload_to_identity(other, id)?,
        )),
    }
}

fn parse_ts(ts: i64) -> Result<NaiveDateTime, DomainError> {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|dt| dt.naive_utc())
        .ok_or_else(|| DomainError::InfrastructureError(format!("invalid timestamp: {ts}")))
}

impl From<&DomainEvent> for EventPayload {
    fn from(event: &DomainEvent) -> Self {
        match event {
            DomainEvent::ReviewLogged {
                review_id,
                movie_id,
                user_id,
                rating,
                watched_at,
            } => EventPayload::ReviewLogged {
                review_id: review_id.value().to_string(),
                movie_id: movie_id.value().to_string(),
                user_id: user_id.value().to_string(),
                rating: rating.value(),
                watched_at: watched_at.and_utc().timestamp(),
            },
            DomainEvent::ReviewUpdated {
                review_id,
                movie_id,
                user_id,
                rating,
                watched_at,
            } => EventPayload::ReviewUpdated {
                review_id: review_id.value().to_string(),
                movie_id: movie_id.value().to_string(),
                user_id: user_id.value().to_string(),
                rating: rating.value(),
                watched_at: watched_at.and_utc().timestamp(),
            },
            DomainEvent::MovieDiscovered {
                movie_id,
                external_metadata_id,
            } => EventPayload::MovieDiscovered {
                movie_id: movie_id.value().to_string(),
                external_metadata_id: external_metadata_id.value().to_owned(),
            },
            DomainEvent::MovieDeleted {
                movie_id,
                poster_path,
            } => EventPayload::MovieDeleted {
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
            DomainEvent::MovieEnrichmentRequested {
                movie_id,
                external_metadata_id,
            } => EventPayload::MovieEnrichmentRequested {
                movie_id: movie_id.value().to_string(),
                external_metadata_id: external_metadata_id.value().to_string(),
            },
            DomainEvent::ImageStored { key } => EventPayload::ImageStored { key: key.clone() },
            DomainEvent::WatchlistEntryAdded {
                user_id,
                movie_id,
                movie_title,
                release_year,
                external_metadata_id,
                added_at,
            } => EventPayload::WatchlistEntryAdded {
                user_id: user_id.value().to_string(),
                movie_id: movie_id.value().to_string(),
                movie_title: movie_title.clone(),
                release_year: *release_year,
                external_metadata_id: external_metadata_id.clone(),
                added_at: added_at.and_utc().timestamp(),
            },
            DomainEvent::WatchlistEntryRemoved { user_id, movie_id } => {
                EventPayload::WatchlistEntryRemoved {
                    user_id: user_id.value().to_string(),
                    movie_id: movie_id.value().to_string(),
                }
            }
            DomainEvent::FollowRequested { follower, target } => {
                let (kind, id) = follow_target_to_payload(target);
                EventPayload::FollowRequested {
                    follower_id: follower.value().to_string(),
                    target_kind: kind,
                    target_id: id,
                }
            }
            DomainEvent::FollowAccepted { owner, requester } => {
                let (kind, id) = identity_to_payload(requester);
                EventPayload::FollowAccepted {
                    owner_id: owner.value().to_string(),
                    requester_kind: kind,
                    requester_id: id,
                }
            }
            DomainEvent::Unfollowed { follower, target } => {
                let (kind, id) = identity_to_payload(target);
                EventPayload::Unfollowed {
                    follower_id: follower.value().to_string(),
                    target_kind: kind,
                    target_id: id,
                }
            }
            DomainEvent::FollowerRemoved { owner, follower } => {
                let (kind, id) = identity_to_payload(follower);
                EventPayload::FollowerRemoved {
                    owner_id: owner.value().to_string(),
                    follower_kind: kind,
                    follower_id: id,
                }
            }
            DomainEvent::ActorBlocked { blocker, target } => {
                let (kind, id) = identity_to_payload(target);
                EventPayload::ActorBlocked {
                    blocker_id: blocker.value().to_string(),
                    target_kind: kind,
                    target_id: id,
                }
            }
            DomainEvent::ActorUnblocked { blocker, target } => {
                let (kind, id) = identity_to_payload(target);
                EventPayload::ActorUnblocked {
                    blocker_id: blocker.value().to_string(),
                    target_kind: kind,
                    target_id: id,
                }
            }
            DomainEvent::BackfillFollower {
                owner_user_id,
                follower_inbox_url,
            } => EventPayload::BackfillFollower {
                owner_user_id: owner_user_id.value().to_string(),
                follower_inbox_url: follower_inbox_url.clone(),
            },
            DomainEvent::FederationDeliveryRequested {
                inbox_url,
                activity_json,
                signing_actor_id,
            } => EventPayload::FederationDeliveryRequested {
                inbox_url: inbox_url.clone(),
                activity_json: activity_json.clone(),
                signing_actor_id: signing_actor_id.to_string(),
            },
            DomainEvent::WatchEventIngested {
                user_id,
                title,
                source,
            } => EventPayload::WatchEventIngested {
                user_id: user_id.value().to_string(),
                title: title.clone(),
                source: source.clone(),
            },
            DomainEvent::WrapUpRequested {
                wrapup_id,
                user_id,
                start_date,
                end_date,
            } => EventPayload::WrapUpRequested {
                wrapup_id: wrapup_id.value().to_string(),
                user_id: user_id.as_ref().map(|u| u.value().to_string()),
                start_date: start_date.to_string(),
                end_date: end_date.to_string(),
            },
            DomainEvent::WrapUpCompleted { wrapup_id } => EventPayload::WrapUpCompleted {
                wrapup_id: wrapup_id.value().to_string(),
            },
            DomainEvent::SearchReindexRequested => EventPayload::SearchReindexRequested,
            DomainEvent::PosterSynced { movie_id } => EventPayload::PosterSynced {
                movie_id: movie_id.value().to_string(),
            },
            DomainEvent::GoalCreated {
                goal_id,
                user_id,
                year,
                target_count,
            } => EventPayload::GoalCreated {
                goal_id: goal_id.value().to_string(),
                user_id: user_id.value().to_string(),
                year: *year,
                target_count: *target_count,
            },
            DomainEvent::GoalUpdated {
                goal_id,
                user_id,
                year,
                target_count,
            } => EventPayload::GoalUpdated {
                goal_id: goal_id.value().to_string(),
                user_id: user_id.value().to_string(),
                year: *year,
                target_count: *target_count,
            },
            DomainEvent::GoalDeleted {
                goal_id,
                user_id,
                year,
            } => EventPayload::GoalDeleted {
                goal_id: goal_id.value().to_string(),
                user_id: user_id.value().to_string(),
                year: *year,
            },
            DomainEvent::PersonEnrichmentRequested {
                person_id,
                external_person_id,
            } => EventPayload::PersonEnrichmentRequested {
                person_id: person_id.value().to_string(),
                external_person_id: external_person_id.value().to_string(),
            },
            DomainEvent::UserDeleted { user_id } => EventPayload::UserDeleted {
                user_id: user_id.value().to_string(),
            },
            DomainEvent::UserAccountMoved {
                user_id,
                new_actor_url,
            } => EventPayload::UserAccountMoved {
                user_id: user_id.value().to_string(),
                new_actor_url: new_actor_url.clone(),
            },
        }
    }
}

impl TryFrom<EventPayload> for DomainEvent {
    type Error = DomainError;
    fn try_from(payload: EventPayload) -> Result<Self, DomainError> {
        match payload {
            EventPayload::ReviewLogged {
                review_id,
                movie_id,
                user_id,
                rating,
                watched_at,
            } => Ok(DomainEvent::ReviewLogged {
                review_id: ReviewId::from_uuid(parse_uuid(&review_id, "review_id")?),
                movie_id: MovieId::from_uuid(parse_uuid(&movie_id, "movie_id")?),
                user_id: UserId::from_uuid(parse_uuid(&user_id, "user_id")?),
                rating: Rating::new(rating)?,
                watched_at: parse_ts(watched_at)?,
            }),
            EventPayload::ReviewUpdated {
                review_id,
                movie_id,
                user_id,
                rating,
                watched_at,
            } => Ok(DomainEvent::ReviewUpdated {
                review_id: ReviewId::from_uuid(parse_uuid(&review_id, "review_id")?),
                movie_id: MovieId::from_uuid(parse_uuid(&movie_id, "movie_id")?),
                user_id: UserId::from_uuid(parse_uuid(&user_id, "user_id")?),
                rating: Rating::new(rating)?,
                watched_at: parse_ts(watched_at)?,
            }),
            EventPayload::MovieDiscovered {
                movie_id,
                external_metadata_id,
            } => Ok(DomainEvent::MovieDiscovered {
                movie_id: MovieId::from_uuid(parse_uuid(&movie_id, "movie_id")?),
                external_metadata_id: ExternalMetadataId::new(external_metadata_id)?,
            }),
            EventPayload::MovieDeleted {
                movie_id,
                poster_path,
            } => {
                let movie_id = MovieId::from_uuid(parse_uuid(&movie_id, "movie_id")?);
                let poster_path = poster_path
                    .map(PosterPath::new)
                    .transpose()
                    .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
                Ok(DomainEvent::MovieDeleted {
                    movie_id,
                    poster_path,
                })
            }
            EventPayload::UserUpdated { user_id } => Ok(DomainEvent::UserUpdated {
                user_id: UserId::from_uuid(parse_uuid(&user_id, "user_id")?),
            }),
            EventPayload::ReviewDeleted { review_id, user_id } => Ok(DomainEvent::ReviewDeleted {
                review_id: ReviewId::from_uuid(parse_uuid(&review_id, "review_id")?),
                user_id: UserId::from_uuid(parse_uuid(&user_id, "user_id")?),
            }),
            EventPayload::MovieEnrichmentRequested {
                movie_id,
                external_metadata_id,
            } => Ok(DomainEvent::MovieEnrichmentRequested {
                movie_id: MovieId::from_uuid(parse_uuid(&movie_id, "movie_id")?),
                external_metadata_id: ExternalMetadataId::new(external_metadata_id)
                    .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
            }),
            EventPayload::ImageStored { key } => Ok(DomainEvent::ImageStored { key }),
            EventPayload::WatchlistEntryAdded {
                user_id,
                movie_id,
                movie_title,
                release_year,
                external_metadata_id,
                added_at,
            } => Ok(DomainEvent::WatchlistEntryAdded {
                user_id: UserId::from_uuid(parse_uuid(&user_id, "user_id")?),
                movie_id: MovieId::from_uuid(parse_uuid(&movie_id, "movie_id")?),
                movie_title,
                release_year,
                external_metadata_id,
                added_at: parse_ts(added_at)?,
            }),
            EventPayload::WatchlistEntryRemoved { user_id, movie_id } => {
                Ok(DomainEvent::WatchlistEntryRemoved {
                    user_id: UserId::from_uuid(parse_uuid(&user_id, "user_id")?),
                    movie_id: MovieId::from_uuid(parse_uuid(&movie_id, "movie_id")?),
                })
            }
            EventPayload::FollowRequested {
                follower_id,
                target_kind,
                target_id,
            } => Ok(DomainEvent::FollowRequested {
                follower: UserId::from_uuid(parse_uuid(&follower_id, "follower_id")?),
                target: payload_to_follow_target(&target_kind, target_id)?,
            }),
            EventPayload::FollowAccepted {
                owner_id,
                requester_kind,
                requester_id,
            } => Ok(DomainEvent::FollowAccepted {
                owner: UserId::from_uuid(parse_uuid(&owner_id, "owner_id")?),
                requester: payload_to_identity(&requester_kind, requester_id)?,
            }),
            EventPayload::Unfollowed {
                follower_id,
                target_kind,
                target_id,
            } => Ok(DomainEvent::Unfollowed {
                follower: UserId::from_uuid(parse_uuid(&follower_id, "follower_id")?),
                target: payload_to_identity(&target_kind, target_id)?,
            }),
            EventPayload::FollowerRemoved {
                owner_id,
                follower_kind,
                follower_id,
            } => Ok(DomainEvent::FollowerRemoved {
                owner: UserId::from_uuid(parse_uuid(&owner_id, "owner_id")?),
                follower: payload_to_identity(&follower_kind, follower_id)?,
            }),
            EventPayload::ActorBlocked {
                blocker_id,
                target_kind,
                target_id,
            } => Ok(DomainEvent::ActorBlocked {
                blocker: UserId::from_uuid(parse_uuid(&blocker_id, "blocker_id")?),
                target: payload_to_identity(&target_kind, target_id)?,
            }),
            EventPayload::ActorUnblocked {
                blocker_id,
                target_kind,
                target_id,
            } => Ok(DomainEvent::ActorUnblocked {
                blocker: UserId::from_uuid(parse_uuid(&blocker_id, "blocker_id")?),
                target: payload_to_identity(&target_kind, target_id)?,
            }),
            EventPayload::BackfillFollower {
                owner_user_id,
                follower_inbox_url,
            } => Ok(DomainEvent::BackfillFollower {
                owner_user_id: UserId::from_uuid(parse_uuid(&owner_user_id, "owner_user_id")?),
                follower_inbox_url,
            }),
            EventPayload::FederationDeliveryRequested {
                inbox_url,
                activity_json,
                signing_actor_id,
            } => Ok(DomainEvent::FederationDeliveryRequested {
                inbox_url,
                activity_json,
                signing_actor_id: parse_uuid(&signing_actor_id, "signing_actor_id")?,
            }),
            EventPayload::WatchEventIngested {
                user_id,
                title,
                source,
            } => Ok(DomainEvent::WatchEventIngested {
                user_id: UserId::from_uuid(parse_uuid(&user_id, "user_id")?),
                title,
                source,
            }),
            EventPayload::WrapUpRequested {
                wrapup_id,
                user_id,
                start_date,
                end_date,
            } => {
                let wid = parse_uuid(&wrapup_id, "wrapup_id")?;
                let uid = user_id.map(|s| parse_uuid(&s, "user_id")).transpose()?;
                let sd = chrono::NaiveDate::parse_from_str(&start_date, "%Y-%m-%d")
                    .map_err(|e| DomainError::ValidationError(e.to_string()))?;
                let ed = chrono::NaiveDate::parse_from_str(&end_date, "%Y-%m-%d")
                    .map_err(|e| DomainError::ValidationError(e.to_string()))?;
                Ok(DomainEvent::WrapUpRequested {
                    wrapup_id: WrapUpId::from_uuid(wid),
                    user_id: uid.map(UserId::from_uuid),
                    start_date: sd,
                    end_date: ed,
                })
            }
            EventPayload::WrapUpCompleted { wrapup_id } => {
                let wid = parse_uuid(&wrapup_id, "wrapup_id")?;
                Ok(DomainEvent::WrapUpCompleted {
                    wrapup_id: WrapUpId::from_uuid(wid),
                })
            }
            EventPayload::SearchReindexRequested => Ok(DomainEvent::SearchReindexRequested),
            EventPayload::PosterSynced { movie_id } => Ok(DomainEvent::PosterSynced {
                movie_id: MovieId::from_uuid(parse_uuid(&movie_id, "movie_id")?),
            }),
            EventPayload::GoalCreated {
                goal_id,
                user_id,
                year,
                target_count,
            } => Ok(DomainEvent::GoalCreated {
                goal_id: GoalId::from_uuid(parse_uuid(&goal_id, "goal_id")?),
                user_id: UserId::from_uuid(parse_uuid(&user_id, "user_id")?),
                year,
                target_count,
            }),
            EventPayload::GoalUpdated {
                goal_id,
                user_id,
                year,
                target_count,
            } => Ok(DomainEvent::GoalUpdated {
                goal_id: GoalId::from_uuid(parse_uuid(&goal_id, "goal_id")?),
                user_id: UserId::from_uuid(parse_uuid(&user_id, "user_id")?),
                year,
                target_count,
            }),
            EventPayload::GoalDeleted {
                goal_id,
                user_id,
                year,
            } => Ok(DomainEvent::GoalDeleted {
                goal_id: GoalId::from_uuid(parse_uuid(&goal_id, "goal_id")?),
                user_id: UserId::from_uuid(parse_uuid(&user_id, "user_id")?),
                year,
            }),
            EventPayload::PersonEnrichmentRequested {
                person_id,
                external_person_id,
            } => Ok(DomainEvent::PersonEnrichmentRequested {
                person_id: PersonId::from_uuid(parse_uuid(&person_id, "person_id")?),
                external_person_id: ExternalPersonId::new(external_person_id),
            }),
            EventPayload::UserDeleted { user_id } => Ok(DomainEvent::UserDeleted {
                user_id: UserId::from_uuid(parse_uuid(&user_id, "user_id")?),
            }),
            EventPayload::UserAccountMoved {
                user_id,
                new_actor_url,
            } => Ok(DomainEvent::UserAccountMoved {
                user_id: UserId::from_uuid(parse_uuid(&user_id, "user_id")?),
                new_actor_url,
            }),
        }
    }
}

#[cfg(test)]
#[path = "tests/lib.rs"]
mod tests;
