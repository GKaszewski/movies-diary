use activitypub_federation::{
    config::Data,
    fetch::object_id::ObjectId,
    protocol::verification::verify_domains_match,
    traits::Object,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

use domain::models::{Review, ReviewSource};
use domain::value_objects::ReviewId;

use crate::actors::DbActor;
use crate::data::FederationData;
use crate::error::Error;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewObject {
    #[serde(rename = "type")]
    pub(crate) kind: String,
    pub(crate) id: ObjectId<DbReview>,
    pub(crate) attributed_to: ObjectId<DbActor>,
    pub(crate) content: String,
    pub(crate) published: DateTime<Utc>,
    pub(crate) movie_title: String,
    pub(crate) rating: u8,
    pub(crate) comment: Option<String>,
    pub(crate) watched_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DbReview {
    pub review: Review,
    pub ap_id: Url,
}

pub fn review_url(base_url: &str, review_id: &ReviewId) -> Url {
    Url::parse(&format!("{}/reviews/{}", base_url, review_id.value())).expect("valid review url")
}

#[async_trait::async_trait]
impl Object for DbReview {
    type DataType = FederationData;
    type Kind = ReviewObject;
    type Error = Error;

    fn id(&self) -> &Url {
        &self.ap_id
    }

    async fn read_from_id(
        _object_id: Url,
        _data: &Data<Self::DataType>,
    ) -> Result<Option<Self>, Self::Error> {
        // Incoming activities provide the full object; no need to dereference local reviews
        Ok(None)
    }

    async fn into_json(self, data: &Data<Self::DataType>) -> Result<Self::Kind, Self::Error> {
        let r = &self.review;
        let ap_id = review_url(&data.base_url, r.id());
        let actor_url = crate::actors::actor_url(&data.base_url, r.user_id());

        let stars: String = "\u{2B50}".repeat(r.rating().value() as usize);
        let comment_text = r.comment().map(|c| c.value().to_string());
        // TODO(ap): fetch movie title from MovieRepository via FederationData
        let movie_title = "Unknown".to_string();

        let fallback = match &comment_text {
            Some(c) => format!("{} Watched '{}': {}", stars, movie_title, c),
            None => format!("{} Watched '{}'", stars, movie_title),
        };

        Ok(ReviewObject {
            kind: "Review".to_string(),
            id: ap_id.into(),
            attributed_to: actor_url.into(),
            content: fallback,
            published: DateTime::from_naive_utc_and_offset(*r.created_at(), Utc),
            movie_title,
            rating: r.rating().value(),
            comment: comment_text,
            watched_at: DateTime::from_naive_utc_and_offset(*r.watched_at(), Utc),
        })
    }

    async fn verify(
        json: &Self::Kind,
        expected_domain: &Url,
        _data: &Data<Self::DataType>,
    ) -> Result<(), Self::Error> {
        verify_domains_match(json.attributed_to.inner(), expected_domain)?;
        Ok(())
    }

    async fn from_json(
        json: Self::Kind,
        data: &Data<Self::DataType>,
    ) -> Result<Self, Self::Error> {
        let actor_url = json.attributed_to.inner().to_string();

        let review_id = ReviewId::generate();
        // TODO(ap): create stub movie/user entries in DB so feed JOIN queries work.
        // For now, use deterministic UUIDs from content hash; reviews will be orphaned in JOINs.
        let movie_id_uuid = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, json.movie_title.as_bytes());
        let movie_id = domain::value_objects::MovieId::from_uuid(movie_id_uuid);
        let user_id_uuid = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, actor_url.as_bytes());
        let user_id = domain::value_objects::UserId::from_uuid(user_id_uuid);
        let rating = domain::value_objects::Rating::new(json.rating.min(5))
            .map_err(|e| Error(anyhow::anyhow!("{}", e)))?;
        let comment = json
            .comment
            .map(|c| domain::value_objects::Comment::new(c))
            .transpose()
            .map_err(|e| Error(anyhow::anyhow!("{}", e)))?;
        let watched_at = json.watched_at.naive_utc();
        let created_at = json.published.naive_utc();

        let review = Review::from_persistence(
            review_id,
            movie_id,
            user_id,
            rating,
            comment,
            watched_at,
            created_at,
            ReviewSource::Remote { actor_url },
        );

        let ap_id = review_url(&data.base_url, review.id());
        data.federation_repo.save_remote_review(&review).await?;

        Ok(DbReview { review, ap_id })
    }
}
