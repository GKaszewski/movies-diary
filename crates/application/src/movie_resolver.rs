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
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use domain::{
        errors::DomainError,
        models::Movie,
        ports::{MetadataSearchCriteria, MovieRepository},
        value_objects::{ExternalMetadataId, MovieId, MovieTitle, PosterUrl, ReleaseYear},
    };

    fn make_cmd(ext_id: Option<&str>, title: Option<&str>, year: Option<u16>) -> LogReviewCommand {
        LogReviewCommand {
            external_metadata_id: ext_id.map(String::from),
            manual_title: title.map(String::from),
            manual_release_year: year,
            manual_director: None,
            user_id: uuid::Uuid::new_v4(),
            rating: 4,
            comment: None,
            watched_at: NaiveDate::from_ymd_opt(2024, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
        }
    }

    fn make_movie() -> Movie {
        Movie::new(
            None,
            MovieTitle::new("Inception".to_string()).unwrap(),
            ReleaseYear::new(2010).unwrap(),
            None,
            None,
        )
    }

    struct RepoWithExternalMovie(Movie);
    struct RepoEmpty;
    struct RepoWithTitleMatch(Movie);

    #[async_trait]
    impl MovieRepository for RepoWithExternalMovie {
        async fn get_movie_by_external_id(
            &self,
            _: &ExternalMetadataId,
        ) -> Result<Option<Movie>, DomainError> {
            Ok(Some(self.0.clone()))
        }
        async fn get_movie_by_id(&self, _: &MovieId) -> Result<Option<Movie>, DomainError> {
            panic!("unexpected")
        }
        async fn get_movies_by_title_and_year(
            &self,
            _: &MovieTitle,
            _: &ReleaseYear,
        ) -> Result<Vec<Movie>, DomainError> {
            panic!("unexpected")
        }
        async fn upsert_movie(&self, _: &Movie) -> Result<(), DomainError> {
            panic!("unexpected")
        }
        async fn delete_movie(&self, _: &MovieId) -> Result<(), DomainError> {
            panic!("unexpected")
        }
    }

    #[async_trait]
    impl MovieRepository for RepoEmpty {
        async fn get_movie_by_external_id(
            &self,
            _: &ExternalMetadataId,
        ) -> Result<Option<Movie>, DomainError> {
            Ok(None)
        }
        async fn get_movie_by_id(&self, _: &MovieId) -> Result<Option<Movie>, DomainError> {
            panic!("unexpected")
        }
        async fn get_movies_by_title_and_year(
            &self,
            _: &MovieTitle,
            _: &ReleaseYear,
        ) -> Result<Vec<Movie>, DomainError> {
            Ok(vec![])
        }
        async fn upsert_movie(&self, _: &Movie) -> Result<(), DomainError> {
            panic!("unexpected")
        }
        async fn delete_movie(&self, _: &MovieId) -> Result<(), DomainError> {
            panic!("unexpected")
        }
    }

    #[async_trait]
    impl MovieRepository for RepoWithTitleMatch {
        async fn get_movie_by_external_id(
            &self,
            _: &ExternalMetadataId,
        ) -> Result<Option<Movie>, DomainError> {
            panic!("unexpected")
        }
        async fn get_movie_by_id(&self, _: &MovieId) -> Result<Option<Movie>, DomainError> {
            panic!("unexpected")
        }
        async fn get_movies_by_title_and_year(
            &self,
            _: &MovieTitle,
            _: &ReleaseYear,
        ) -> Result<Vec<Movie>, DomainError> {
            Ok(vec![self.0.clone()])
        }
        async fn upsert_movie(&self, _: &Movie) -> Result<(), DomainError> {
            panic!("unexpected")
        }
        async fn delete_movie(&self, _: &MovieId) -> Result<(), DomainError> {
            panic!("unexpected")
        }
    }

    struct MetaReturnsMovie(Movie);
    struct MetaErrors;

    #[async_trait]
    impl MetadataClient for MetaReturnsMovie {
        async fn fetch_movie_metadata(
            &self,
            _: &MetadataSearchCriteria,
        ) -> Result<Movie, DomainError> {
            Ok(self.0.clone())
        }
        async fn get_poster_url(
            &self,
            _: &ExternalMetadataId,
        ) -> Result<Option<PosterUrl>, DomainError> {
            panic!("unexpected")
        }
    }

    #[async_trait]
    impl MetadataClient for MetaErrors {
        async fn fetch_movie_metadata(
            &self,
            _: &MetadataSearchCriteria,
        ) -> Result<Movie, DomainError> {
            Err(DomainError::InfrastructureError(
                "metadata unavailable".into(),
            ))
        }
        async fn get_poster_url(
            &self,
            _: &ExternalMetadataId,
        ) -> Result<Option<PosterUrl>, DomainError> {
            panic!("unexpected")
        }
    }

    // --- ExternalIdStrategy ---

    #[test]
    fn external_id_strategy_can_handle_cmd_with_id() {
        let cmd = make_cmd(Some("tt123"), None, None);
        assert!(ExternalIdStrategy.can_handle(&cmd));
    }

    #[test]
    fn external_id_strategy_cannot_handle_cmd_without_id() {
        let cmd = make_cmd(None, Some("Inception"), Some(2010));
        assert!(!ExternalIdStrategy.can_handle(&cmd));
    }

    #[tokio::test]
    async fn external_id_strategy_returns_cached_movie() {
        let movie = make_movie();
        let repo = RepoWithExternalMovie(movie.clone());
        let meta = MetaErrors;
        let deps = MovieResolverDeps {
            repository: &repo,
            metadata_client: &meta,
        };
        let cmd = make_cmd(Some("tt123"), None, None);
        let result = ExternalIdStrategy.resolve(&cmd, &deps).await.unwrap();
        assert!(matches!(result, Some((_, false))));
    }

    #[tokio::test]
    async fn external_id_strategy_fetches_from_metadata_when_not_cached() {
        let movie = make_movie();
        let repo = RepoEmpty;
        let meta = MetaReturnsMovie(movie);
        let deps = MovieResolverDeps {
            repository: &repo,
            metadata_client: &meta,
        };
        let cmd = make_cmd(Some("tt123"), None, None);
        let result = ExternalIdStrategy.resolve(&cmd, &deps).await.unwrap();
        assert!(matches!(result, Some((_, true))));
    }

    #[tokio::test]
    async fn external_id_strategy_falls_through_on_metadata_error() {
        let repo = RepoEmpty;
        let meta = MetaErrors;
        let deps = MovieResolverDeps {
            repository: &repo,
            metadata_client: &meta,
        };
        let cmd = make_cmd(Some("tt123"), None, None);
        let result = ExternalIdStrategy.resolve(&cmd, &deps).await.unwrap();
        assert!(result.is_none());
    }

    // --- TitleSearchStrategy ---

    #[test]
    fn title_strategy_can_handle_cmd_with_title() {
        let cmd = make_cmd(None, Some("Inception"), Some(2010));
        assert!(TitleSearchStrategy.can_handle(&cmd));
    }

    #[test]
    fn title_strategy_cannot_handle_cmd_without_title() {
        let cmd = make_cmd(Some("tt123"), None, None);
        assert!(!TitleSearchStrategy.can_handle(&cmd));
    }

    #[tokio::test]
    async fn title_strategy_fetches_from_metadata() {
        let movie = make_movie();
        let repo = RepoEmpty;
        let meta = MetaReturnsMovie(movie);
        let deps = MovieResolverDeps {
            repository: &repo,
            metadata_client: &meta,
        };
        let cmd = make_cmd(None, Some("Inception"), Some(2010));
        let result = TitleSearchStrategy.resolve(&cmd, &deps).await.unwrap();
        assert!(matches!(result, Some((_, true))));
    }

    #[tokio::test]
    async fn title_strategy_falls_through_on_metadata_error() {
        let repo = RepoEmpty;
        let meta = MetaErrors;
        let deps = MovieResolverDeps {
            repository: &repo,
            metadata_client: &meta,
        };
        let cmd = make_cmd(None, Some("Inception"), Some(2010));
        let result = TitleSearchStrategy.resolve(&cmd, &deps).await.unwrap();
        assert!(result.is_none());
    }

    // --- ManualMovieStrategy ---

    #[test]
    fn manual_strategy_can_handle_cmd_with_title() {
        let cmd = make_cmd(None, Some("Inception"), Some(2010));
        assert!(ManualMovieStrategy.can_handle(&cmd));
    }

    #[test]
    fn manual_strategy_cannot_handle_cmd_without_title() {
        let cmd = make_cmd(Some("tt123"), None, None);
        assert!(!ManualMovieStrategy.can_handle(&cmd));
    }

    #[tokio::test]
    async fn manual_strategy_returns_existing_movie() {
        let movie = make_movie();
        let repo = RepoWithTitleMatch(movie.clone());
        let meta = MetaErrors;
        let deps = MovieResolverDeps {
            repository: &repo,
            metadata_client: &meta,
        };
        let cmd = make_cmd(None, Some("Inception"), Some(2010));
        let result = ManualMovieStrategy.resolve(&cmd, &deps).await.unwrap();
        assert!(matches!(result, Some((_, false))));
    }

    #[tokio::test]
    async fn manual_strategy_creates_new_movie_when_no_match() {
        let repo = RepoEmpty;
        let meta = MetaErrors;
        let deps = MovieResolverDeps {
            repository: &repo,
            metadata_client: &meta,
        };
        let cmd = make_cmd(None, Some("Inception"), Some(2010));
        let result = ManualMovieStrategy.resolve(&cmd, &deps).await.unwrap();
        assert!(matches!(result, Some((_, true))));
    }

    #[tokio::test]
    async fn manual_strategy_errors_without_year() {
        let repo = RepoEmpty;
        let meta = MetaErrors;
        let deps = MovieResolverDeps {
            repository: &repo,
            metadata_client: &meta,
        };
        let cmd = make_cmd(None, Some("Inception"), None);
        assert!(ManualMovieStrategy.resolve(&cmd, &deps).await.is_err());
    }

    // --- MovieResolver pipeline ---

    #[tokio::test]
    async fn resolver_returns_error_when_no_strategy_matches() {
        let repo = RepoEmpty;
        let meta = MetaErrors;
        let deps = MovieResolverDeps {
            repository: &repo,
            metadata_client: &meta,
        };
        let cmd = make_cmd(None, None, None);
        let result = MovieResolver::default_pipeline().resolve(&cmd, &deps).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn resolver_uses_cached_movie_when_external_id_matches() {
        let movie = make_movie();
        let repo = RepoWithExternalMovie(movie.clone());
        let meta = MetaErrors;
        let deps = MovieResolverDeps {
            repository: &repo,
            metadata_client: &meta,
        };
        let cmd = make_cmd(Some("tt123"), None, None);
        let (_, is_new) = MovieResolver::default_pipeline()
            .resolve(&cmd, &deps)
            .await
            .unwrap();
        assert!(!is_new);
    }

    #[tokio::test]
    async fn resolver_falls_through_to_manual_when_external_and_title_both_fail() {
        let repo = RepoEmpty;
        let meta = MetaErrors;
        let deps = MovieResolverDeps {
            repository: &repo,
            metadata_client: &meta,
        };
        let cmd = make_cmd(Some("tt123"), Some("Inception"), Some(2010));
        let (_, is_new) = MovieResolver::default_pipeline()
            .resolve(&cmd, &deps)
            .await
            .unwrap();
        assert!(is_new);
    }
}
