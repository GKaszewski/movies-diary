use super::*;
use crate::diary::commands::MovieInput;
use domain::{
    errors::DomainError,
    models::{MetadataSearchCriteria, Movie},
    ports::MovieRepository,
    value_objects::{ExternalMetadataId, MovieId, MovieTitle, PosterUrl, ReleaseYear},
};

fn make_input(ext_id: Option<&str>, title: Option<&str>, year: Option<u16>) -> MovieInput {
    MovieInput {
        movie_id: None,
        external_metadata_id: ext_id.map(String::from),
        manual_title: title.map(String::from),
        manual_release_year: year,
        manual_director: None,
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

#[async_trait::async_trait]
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
    async fn existing_external_ids(
        &self,
        _: &[ExternalMetadataId],
    ) -> Result<std::collections::HashSet<String>, DomainError> {
        panic!("unexpected")
    }
    async fn existing_title_year_pairs(
        &self,
        _: &[(MovieTitle, ReleaseYear)],
    ) -> Result<std::collections::HashSet<(String, u16)>, DomainError> {
        panic!("unexpected")
    }
    async fn list_movies(
        &self,
        _: &domain::models::collections::PageParams,
        _: &domain::models::MovieFilter,
    ) -> Result<domain::models::collections::Paginated<domain::models::MovieSummary>, DomainError>
    {
        panic!("unexpected")
    }
}

#[async_trait::async_trait]
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
    async fn existing_external_ids(
        &self,
        _: &[ExternalMetadataId],
    ) -> Result<std::collections::HashSet<String>, DomainError> {
        panic!("unexpected")
    }
    async fn existing_title_year_pairs(
        &self,
        _: &[(MovieTitle, ReleaseYear)],
    ) -> Result<std::collections::HashSet<(String, u16)>, DomainError> {
        panic!("unexpected")
    }
    async fn list_movies(
        &self,
        _: &domain::models::collections::PageParams,
        _: &domain::models::MovieFilter,
    ) -> Result<domain::models::collections::Paginated<domain::models::MovieSummary>, DomainError>
    {
        panic!("unexpected")
    }
}

#[async_trait::async_trait]
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
    async fn existing_external_ids(
        &self,
        _: &[ExternalMetadataId],
    ) -> Result<std::collections::HashSet<String>, DomainError> {
        panic!("unexpected")
    }
    async fn existing_title_year_pairs(
        &self,
        _: &[(MovieTitle, ReleaseYear)],
    ) -> Result<std::collections::HashSet<(String, u16)>, DomainError> {
        panic!("unexpected")
    }
    async fn list_movies(
        &self,
        _: &domain::models::collections::PageParams,
        _: &domain::models::MovieFilter,
    ) -> Result<domain::models::collections::Paginated<domain::models::MovieSummary>, DomainError>
    {
        panic!("unexpected")
    }
}

struct MetaReturnsMovie(Movie);
struct MetaErrors;

#[async_trait::async_trait]
impl MetadataClient for MetaReturnsMovie {
    async fn fetch_movie_metadata(&self, _: &MetadataSearchCriteria) -> Result<Movie, DomainError> {
        Ok(self.0.clone())
    }
    async fn get_poster_url(
        &self,
        _: &ExternalMetadataId,
    ) -> Result<Option<PosterUrl>, DomainError> {
        panic!("unexpected")
    }
}

#[async_trait::async_trait]
impl MetadataClient for MetaErrors {
    async fn fetch_movie_metadata(&self, _: &MetadataSearchCriteria) -> Result<Movie, DomainError> {
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
    let input = make_input(Some("tt123"), None, None);
    assert!(ExternalIdStrategy.can_handle(&input));
}

#[test]
fn external_id_strategy_cannot_handle_cmd_without_id() {
    let input = make_input(None, Some("Inception"), Some(2010));
    assert!(!ExternalIdStrategy.can_handle(&input));
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
    let input = make_input(Some("tt123"), None, None);
    let result = ExternalIdStrategy.resolve(&input, &deps).await.unwrap();
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
    let input = make_input(Some("tt123"), None, None);
    let result = ExternalIdStrategy.resolve(&input, &deps).await.unwrap();
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
    let input = make_input(Some("tt123"), None, None);
    let result = ExternalIdStrategy.resolve(&input, &deps).await.unwrap();
    assert!(result.is_none());
}

// --- TitleSearchStrategy ---

#[test]
fn title_strategy_can_handle_cmd_with_title() {
    let input = make_input(None, Some("Inception"), Some(2010));
    assert!(TitleSearchStrategy.can_handle(&input));
}

#[test]
fn title_strategy_cannot_handle_cmd_without_title() {
    let input = make_input(Some("tt123"), None, None);
    assert!(!TitleSearchStrategy.can_handle(&input));
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
    let input = make_input(None, Some("Inception"), Some(2010));
    let result = TitleSearchStrategy.resolve(&input, &deps).await.unwrap();
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
    let input = make_input(None, Some("Inception"), Some(2010));
    let result = TitleSearchStrategy.resolve(&input, &deps).await.unwrap();
    assert!(result.is_none());
}

// --- ManualMovieStrategy ---

#[test]
fn manual_strategy_can_handle_cmd_with_title() {
    let input = make_input(None, Some("Inception"), Some(2010));
    assert!(ManualMovieStrategy.can_handle(&input));
}

#[test]
fn manual_strategy_cannot_handle_cmd_without_title() {
    let input = make_input(Some("tt123"), None, None);
    assert!(!ManualMovieStrategy.can_handle(&input));
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
    let input = make_input(None, Some("Inception"), Some(2010));
    let result = ManualMovieStrategy.resolve(&input, &deps).await.unwrap();
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
    let input = make_input(None, Some("Inception"), Some(2010));
    let result = ManualMovieStrategy.resolve(&input, &deps).await.unwrap();
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
    let input = make_input(None, Some("Inception"), None);
    assert!(ManualMovieStrategy.resolve(&input, &deps).await.is_err());
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
    let input = make_input(None, None, None);
    let result = MovieResolver::default_pipeline()
        .resolve(&input, &deps)
        .await;
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
    let input = make_input(Some("tt123"), None, None);
    let (_, is_new) = MovieResolver::default_pipeline()
        .resolve(&input, &deps)
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
    let input = make_input(Some("tt123"), Some("Inception"), Some(2010));
    let (_, is_new) = MovieResolver::default_pipeline()
        .resolve(&input, &deps)
        .await
        .unwrap();
    assert!(is_new);
}
