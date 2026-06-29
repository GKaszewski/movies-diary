use async_trait::async_trait;
use chrono::Utc;
use domain::{
    errors::DomainError,
    models::{CastMember, CrewMember, Genre, Keyword, MovieProfile},
    ports::MovieEnrichmentClient,
    value_objects::MovieId,
};
use serde::Deserialize;

use crate::client::TmdbEnrichmentClient;

#[async_trait]
impl MovieEnrichmentClient for TmdbEnrichmentClient {
    async fn fetch_profile(
        &self,
        movie_id: MovieId,
        external_metadata_id: &str,
    ) -> Result<MovieProfile, DomainError> {
        let tmdb_id = self.resolve_tmdb_id(external_metadata_id).await?;

        #[derive(Deserialize)]
        struct GenreDto {
            id: u32,
            name: String,
        }
        #[derive(Deserialize)]
        struct CollectionDto {
            name: String,
        }
        #[derive(Deserialize)]
        struct CastDto {
            id: u64,
            name: String,
            character: String,
            order: u32,
            profile_path: Option<String>,
        }
        #[derive(Deserialize)]
        struct CrewDto {
            id: u64,
            name: String,
            job: String,
            department: String,
            profile_path: Option<String>,
        }
        #[derive(Deserialize)]
        struct Credits {
            cast: Vec<CastDto>,
            crew: Vec<CrewDto>,
        }
        #[derive(Deserialize)]
        struct KeywordDto {
            id: u32,
            name: String,
        }
        #[derive(Deserialize)]
        struct Keywords {
            keywords: Vec<KeywordDto>,
        }
        #[derive(Deserialize)]
        struct Details {
            imdb_id: Option<String>,
            overview: Option<String>,
            tagline: Option<String>,
            runtime: Option<u32>,
            budget: Option<i64>,
            revenue: Option<i64>,
            vote_average: Option<f64>,
            vote_count: Option<u32>,
            original_language: Option<String>,
            genres: Vec<GenreDto>,
            belongs_to_collection: Option<CollectionDto>,
            credits: Credits,
            keywords: Keywords,
        }

        let url = self.base(&format!("/movie/{}", tmdb_id));
        let d: Details = self
            .get(&url, &[("append_to_response", "credits,keywords")])
            .await?;

        Ok(MovieProfile {
            movie_id,
            tmdb_id,
            imdb_id: d.imdb_id.filter(|s| !s.is_empty()),
            overview: d.overview.filter(|s| !s.is_empty()),
            tagline: d.tagline.filter(|s| !s.is_empty()),
            runtime_minutes: d.runtime,
            budget_usd: d.budget.filter(|&v| v > 0),
            revenue_usd: d.revenue.filter(|&v| v > 0),
            vote_average: d.vote_average,
            vote_count: d.vote_count,
            original_language: d.original_language,
            collection_name: d.belongs_to_collection.map(|c| c.name),
            genres: d
                .genres
                .into_iter()
                .map(|g| Genre {
                    tmdb_id: g.id,
                    name: g.name,
                })
                .collect(),
            keywords: d
                .keywords
                .keywords
                .into_iter()
                .map(|k| Keyword {
                    tmdb_id: k.id,
                    name: k.name,
                })
                .collect(),
            cast: d
                .credits
                .cast
                .into_iter()
                .map(|c| CastMember {
                    tmdb_person_id: c.id,
                    name: c.name,
                    character: c.character,
                    billing_order: c.order,
                    profile_path: c.profile_path,
                })
                .collect(),
            crew: d
                .credits
                .crew
                .into_iter()
                .map(|c| CrewMember {
                    tmdb_person_id: c.id,
                    name: c.name,
                    job: c.job,
                    department: c.department,
                    profile_path: c.profile_path,
                })
                .collect(),
            enriched_at: Utc::now(),
        })
    }
}
