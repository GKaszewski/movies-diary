use activitypub_federation::{
    config::Data,
    fetch::object_id::ObjectId,
    kinds::object::NoteType,
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
    pub(crate) kind: NoteType,
    pub(crate) id: ObjectId<DbReview>,
    pub(crate) attributed_to: ObjectId<DbActor>,
    pub(crate) content: String,
    pub(crate) published: DateTime<Utc>,
    pub(crate) movie_title: String,
    #[serde(default)]
    pub(crate) release_year: u16,   // 0 = unknown; default for old AP messages
    #[serde(default)]
    pub(crate) poster_url: Option<String>,
    pub(crate) rating: u8,
    pub(crate) comment: Option<String>,
    pub(crate) watched_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DbReview {
    pub review: Review,
    pub ap_id: Url,
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
        let ap_id = crate::urls::review_url(&data.base_url, r.id());
        let actor_url = crate::urls::actor_url(&data.base_url, r.user_id());

        let movie = data.movie_repo.get_movie_by_id(r.movie_id()).await
            .ok().flatten();
        let movie_title = movie.as_ref()
            .map(|m| m.title().value().to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        let release_year = movie.as_ref()
            .map(|m| m.release_year().value())
            .unwrap_or(0);
        let poster_url = movie.as_ref()
            .and_then(|m| m.poster_path())
            .map(|p| format!("{}/posters/{}", data.base_url, p.value()));

        let stars: String = "\u{2B50}".repeat(r.rating().value() as usize);
        let comment_text = r.comment().map(|c| c.value().to_string());
        let year_str = if release_year > 0 { format!(" ({})", release_year) } else { String::new() };
        let watched_str = format!("Watched: {}", r.watched_at().format("%b %-d, %Y"));
        let content = match &comment_text {
            Some(c) => format!("{} {}{}\n{}\n{}", stars, movie_title, year_str, c, watched_str),
            None => format!("{} {}{}\n{}", stars, movie_title, year_str, watched_str),
        };

        Ok(ReviewObject {
            kind: NoteType::default(),
            id: ap_id.into(),
            attributed_to: actor_url.into(),
            content,
            published: DateTime::from_naive_utc_and_offset(*r.created_at(), Utc),
            movie_title,
            release_year,
            poster_url,
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
        let movie_id_uuid = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, json.movie_title.as_bytes());
        let movie_id = domain::value_objects::MovieId::from_uuid(movie_id_uuid);
        let user_id_uuid = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, actor_url.as_bytes());
        let user_id = domain::value_objects::UserId::from_uuid(user_id_uuid);
        let rating = domain::value_objects::Rating::new(json.rating.min(5))
            .map_err(|e| Error::bad_request(anyhow::anyhow!("{}", e)))?;
        let comment = json
            .comment
            .map(|c| domain::value_objects::Comment::new(c))
            .transpose()
            .map_err(|e| Error::bad_request(anyhow::anyhow!("{}", e)))?;
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

        let ap_id_url = json.id.into_inner();
        data.federation_repo.save_remote_review(&review, ap_id_url.as_str(), &json.movie_title, json.release_year, json.poster_url.as_deref()).await?;

        Ok(DbReview { review, ap_id: ap_id_url })
    }
}
