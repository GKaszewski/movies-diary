use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::Movie,
    ports::{MetadataClient, MetadataSearchCriteria, MovieRepository},
    value_objects::{ExternalMetadataId, MovieTitle, ReleaseYear},
};

use crate::commands::LogReviewCommand;

pub struct MovieResolverDeps<'a> {
    pub repository: &'a dyn MovieRepository,
    pub metadata_client: &'a dyn MetadataClient,
}

#[async_trait]
pub trait ResolutionStrategy: Send + Sync {
    fn can_handle(&self, cmd: &LogReviewCommand) -> bool;
    async fn resolve(
        &self,
        cmd: &LogReviewCommand,
        deps: &MovieResolverDeps<'_>,
    ) -> Result<Option<(Movie, bool)>, DomainError>;
}

pub struct ExternalIdStrategy;
pub struct TitleSearchStrategy;
pub struct ManualMovieStrategy;

pub struct MovieResolver {
    strategies: Vec<Box<dyn ResolutionStrategy>>,
}

impl MovieResolver {
    pub fn default_pipeline() -> Self {
        Self {
            strategies: vec![
                Box::new(ExternalIdStrategy),
                Box::new(TitleSearchStrategy),
                Box::new(ManualMovieStrategy),
            ],
        }
    }

    pub async fn resolve(
        &self,
        cmd: &LogReviewCommand,
        deps: &MovieResolverDeps<'_>,
    ) -> Result<(Movie, bool), DomainError> {
        for strategy in &self.strategies {
            if strategy.can_handle(cmd) {
                if let Some(result) = strategy.resolve(cmd, deps).await? {
                    return Ok(result);
                }
            }
        }
        Err(DomainError::ValidationError(
            "Manual title required if TMDB fetch fails or is omitted".into(),
        ))
    }
}

#[async_trait]
impl ResolutionStrategy for ExternalIdStrategy {
    fn can_handle(&self, cmd: &LogReviewCommand) -> bool {
        cmd.external_metadata_id.is_some()
    }

    async fn resolve(
        &self,
        cmd: &LogReviewCommand,
        deps: &MovieResolverDeps<'_>,
    ) -> Result<Option<(Movie, bool)>, DomainError> {
        let ext_id_str = cmd.external_metadata_id.as_deref().unwrap();
        let tmdb_id = ExternalMetadataId::new(ext_id_str.to_string())?;

        if let Some(m) = deps.repository.get_movie_by_external_id(&tmdb_id).await? {
            return Ok(Some((m, false)));
        }

        match deps
            .metadata_client
            .fetch_movie_metadata(&MetadataSearchCriteria::ImdbId(tmdb_id))
            .await
        {
            Ok(m) => Ok(Some((m, true))),
            Err(e) => {
                tracing::warn!(
                    "Failed to fetch from TMDB, falling back to manual entry: {:?}",
                    e
                );
                Ok(None)
            }
        }
    }
}

#[async_trait]
impl ResolutionStrategy for TitleSearchStrategy {
    fn can_handle(&self, cmd: &LogReviewCommand) -> bool {
        cmd.manual_title.is_some()
    }

    async fn resolve(
        &self,
        cmd: &LogReviewCommand,
        deps: &MovieResolverDeps<'_>,
    ) -> Result<Option<(Movie, bool)>, DomainError> {
        let title = cmd.manual_title.as_deref().unwrap();
        let criteria = MetadataSearchCriteria::Title {
            title: MovieTitle::new(title.to_string())?,
            year: cmd.manual_release_year.map(ReleaseYear::new).transpose()?,
        };
        match deps.metadata_client.fetch_movie_metadata(&criteria).await {
            Ok(m) => Ok(Some((m, true))),
            Err(e) => {
                tracing::warn!("OMDb title search failed, falling back to manual: {:?}", e);
                Ok(None)
            }
        }
    }
}

#[async_trait]
impl ResolutionStrategy for ManualMovieStrategy {
    fn can_handle(&self, cmd: &LogReviewCommand) -> bool {
        cmd.manual_title.is_some()
    }

    async fn resolve(
        &self,
        cmd: &LogReviewCommand,
        deps: &MovieResolverDeps<'_>,
    ) -> Result<Option<(Movie, bool)>, DomainError> {
        let title_str = match &cmd.manual_title {
            Some(t) => t,
            None => return Ok(None),
        };
        let year_val = cmd.manual_release_year.ok_or_else(|| {
            DomainError::ValidationError(
                "Manual release year required if TMDB fetch fails or is omitted".into(),
            )
        })?;

        let title = MovieTitle::new(title_str.clone())?;
        let release_year = ReleaseYear::new(year_val)?;

        let candidates = deps
            .repository
            .get_movies_by_title_and_year(&title, &release_year)
            .await?;

        let matched = candidates
            .into_iter()
            .find(|m| m.is_manual_match(&title, &release_year, cmd.manual_director.as_deref()));

        if let Some(existing) = matched {
            Ok(Some((existing, false)))
        } else {
            let new_movie =
                Movie::new(None, title, release_year, cmd.manual_director.clone(), None);
            Ok(Some((new_movie, true)))
        }
    }
}

#[cfg(test)]
#[path = "tests/movie_resolver.rs"]
mod tests;
