// Re-export imports needed by subtest modules
pub use application::{config::AppConfig, context::AppContext};
pub use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    routing::get,
};
pub use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{
        DiaryEntry, DiaryFilter, FeedEntry, Movie, Review, ReviewHistory, UserStats,
        UserTrends,
        collections::{PageParams, Paginated},
        PersonId, EntityType, IndexableDocument, Person, PersonCredits,
        SearchQuery, SearchResults,
    },
    ports::{
        AuthService, DiaryRepository, EventPublisher, GeneratedToken, ImageStorage,
        MetadataClient, MovieRepository, PasswordHasher, PosterFetcherClient, ReviewRepository,
        StatsRepository, UserRepository,
        PersonCommand, PersonQuery, SearchPort, SearchCommand,
    },
    value_objects::{
        Email, ExternalMetadataId, MovieId, MovieTitle, PasswordHash, PosterUrl,
        ReleaseYear, ReviewId, UserId,
    },
};
pub use std::sync::Arc;
pub use tower::ServiceExt;

// API types for tests
pub use api_types::{
    LoginRequest, LogReviewRequest, DiaryQueryParams,
};
pub use crate::{
    extractors::{AuthenticatedUser, OptionalCookieUser, RequiredCookieUser},
    forms::{LogReviewData, LogReviewForm, to_diary_query},
    state::AppState,
};

mod extractors;
mod forms;
mod api_handlers;
