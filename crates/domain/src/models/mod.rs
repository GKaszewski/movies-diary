use chrono::{DateTime, NaiveDateTime, Utc};

use crate::{
    errors::DomainError,
    models::collections::PageParams,
    value_objects::{
        Comment, Email, ExternalMetadataId, MovieId, MovieTitle, PasswordHash, PosterPath, Rating,
        ReleaseYear, ReviewId, UserId, Username,
    },
};
pub mod collections;
pub mod import;
pub mod import_profile;
pub mod import_session;
pub mod person;
pub mod search;
pub mod watchlist;
pub use watchlist::{WatchlistEntry, WatchlistWithMovie};
pub mod remote_watchlist;
pub use remote_watchlist::RemoteWatchlistEntry;

pub use import::{
    AnnotatedRow, DomainField, FieldMapping, FileFormat, ImportError, ImportRow, ParsedFile,
    RowResult, Transform,
};
pub use import_profile::ImportProfile;
pub use import_session::ImportSession;
pub use person::{CastCredit, CrewCredit, ExternalPersonId, Person, PersonCredits, PersonId};
pub use search::{
    EntityType, IndexableDocument, MovieSearchHit, PersonSearchHit, SearchFilters, SearchQuery,
    SearchResults,
};

#[derive(Clone, Debug, Default)]
pub enum SortDirection {
    #[default]
    Descending,
    Ascending,
    ByRatingDesc,
    ByRatingAsc,
}

#[derive(Clone, Debug, Default)]
pub struct DiaryFilter {
    pub sort_by: SortDirection,
    pub page: PageParams,
    pub movie_id: Option<MovieId>,
    pub user_id: Option<UserId>,
    pub search: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct MovieFilter {
    pub search: Option<String>,
    pub genre: Option<String>,
    pub language: Option<String>,
}

#[derive(Clone, Debug)]
pub struct MovieSummary {
    pub movie: Movie,
    pub genres: Vec<String>,
    pub runtime_minutes: Option<u32>,
    pub original_language: Option<String>,
    pub overview: Option<String>,
    pub collection_name: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Movie {
    id: MovieId,
    external_metadata_id: Option<ExternalMetadataId>,
    title: MovieTitle,
    release_year: ReleaseYear,
    director: Option<String>,
    poster_path: Option<PosterPath>,
}

impl Movie {
    pub fn new(
        external_metadata_id: Option<ExternalMetadataId>,
        title: MovieTitle,
        release_year: ReleaseYear,
        director: Option<String>,
        poster_path: Option<PosterPath>,
    ) -> Self {
        Self {
            id: MovieId::generate(),
            external_metadata_id,
            title,
            release_year,
            director,
            poster_path,
        }
    }

    pub fn from_persistence(
        id: MovieId,
        external_metadata_id: Option<ExternalMetadataId>,
        title: MovieTitle,
        release_year: ReleaseYear,
        director: Option<String>,
        poster_path: Option<PosterPath>,
    ) -> Self {
        Self {
            id,
            external_metadata_id,
            title,
            release_year,
            director,
            poster_path,
        }
    }

    pub fn update_poster(&mut self, poster_path: PosterPath) {
        self.poster_path = Some(poster_path);
    }

    pub fn id(&self) -> &MovieId {
        &self.id
    }
    pub fn external_metadata_id(&self) -> Option<&ExternalMetadataId> {
        self.external_metadata_id.as_ref()
    }
    pub fn title(&self) -> &MovieTitle {
        &self.title
    }
    pub fn release_year(&self) -> &ReleaseYear {
        &self.release_year
    }
    pub fn director(&self) -> Option<&str> {
        self.director.as_deref()
    }
    pub fn poster_path(&self) -> Option<&PosterPath> {
        self.poster_path.as_ref()
    }
}

impl Movie {
    pub fn is_manual_match(
        &self,
        title: &MovieTitle,
        year: &ReleaseYear,
        director: Option<&str>,
    ) -> bool {
        if self.title != *title || self.release_year != *year {
            return false;
        }

        match (self.director(), director) {
            (Some(existing_dir), Some(new_dir)) => existing_dir.eq_ignore_ascii_case(new_dir),
            _ => true,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ReviewSource {
    #[default]
    Local,
    Remote {
        actor_url: String,
    },
}

pub struct PersistedReview {
    pub id: ReviewId,
    pub movie_id: MovieId,
    pub user_id: UserId,
    pub rating: Rating,
    pub comment: Option<Comment>,
    pub watched_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub source: ReviewSource,
}

#[derive(Clone, Debug)]
pub struct Review {
    id: ReviewId,
    movie_id: MovieId,
    user_id: UserId,
    rating: Rating,
    comment: Option<Comment>,
    watched_at: chrono::NaiveDateTime,
    created_at: chrono::NaiveDateTime,
    source: ReviewSource,
}

impl Review {
    pub fn new(
        movie_id: MovieId,
        user_id: UserId,
        rating: Rating,
        comment: Option<Comment>,
        watched_at: NaiveDateTime,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            id: ReviewId::generate(),
            movie_id,
            user_id,
            rating,
            comment,
            watched_at,
            created_at: Utc::now().naive_utc(),
            source: ReviewSource::Local,
        })
    }

    pub fn from_persistence(row: PersistedReview) -> Self {
        Self {
            id: row.id,
            movie_id: row.movie_id,
            user_id: row.user_id,
            rating: row.rating,
            comment: row.comment,
            watched_at: row.watched_at,
            created_at: row.created_at,
            source: row.source,
        }
    }

    pub fn id(&self) -> &ReviewId {
        &self.id
    }
    pub fn movie_id(&self) -> &MovieId {
        &self.movie_id
    }
    pub fn user_id(&self) -> &UserId {
        &self.user_id
    }
    pub fn rating(&self) -> &Rating {
        &self.rating
    }
    pub fn comment(&self) -> Option<&Comment> {
        self.comment.as_ref()
    }
    pub fn watched_at(&self) -> &NaiveDateTime {
        &self.watched_at
    }
    pub fn created_at(&self) -> &NaiveDateTime {
        &self.created_at
    }
    pub fn source(&self) -> &ReviewSource {
        &self.source
    }
    /// Returns [star1_filled, star2_filled, ..., star5_filled]
    pub fn stars(&self) -> [bool; 5] {
        let r = self.rating.value();
        [r >= 1, r >= 2, r >= 3, r >= 4, r >= 5]
    }

    pub fn is_remote(&self) -> bool {
        matches!(self.source, ReviewSource::Remote { .. })
    }
}

#[derive(Clone, Debug)]
pub struct DiaryEntry {
    movie: Movie,
    review: Review,
}

impl DiaryEntry {
    pub fn new(movie: Movie, review: Review) -> Self {
        Self { movie, review }
    }

    pub fn movie(&self) -> &Movie {
        &self.movie
    }
    pub fn review(&self) -> &Review {
        &self.review
    }
}

#[derive(Clone, Debug)]
pub struct ReviewHistory {
    movie: Movie,
    viewings: Vec<Review>,
}

impl ReviewHistory {
    pub fn new(movie: Movie, viewings: Vec<Review>) -> Self {
        Self { movie, viewings }
    }

    pub fn movie(&self) -> &Movie {
        &self.movie
    }
    pub fn viewings(&self) -> &[Review] {
        &self.viewings
    }
    pub fn sort_by_date(&mut self) {
        self.viewings.sort_by_key(|r| *r.watched_at());
    }
}

#[derive(Clone, Debug)]
pub struct MovieStats {
    pub total_count: u64,
    pub avg_rating: Option<f64>,
    pub federated_count: u64,
    pub rating_histogram: [u64; 5], // index 0 = 1★, index 4 = 5★
}

#[derive(Clone, Debug, Default)]
pub enum UserRole {
    #[default]
    Standard,
    Admin,
}

#[derive(Debug, Clone)]
pub struct ProfileField {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Default)]
pub struct UserProfile {
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_path: Option<String>,
    pub banner_path: Option<String>,
    pub also_known_as: Option<String>,
    pub profile_fields: Vec<ProfileField>,
}

#[derive(Clone, Debug)]
pub struct User {
    id: UserId,
    email: Email,
    username: Username,
    password_hash: PasswordHash,
    role: UserRole,
    profile: UserProfile,
}

impl User {
    pub fn new(
        email: Email,
        username: Username,
        password_hash: PasswordHash,
        role: UserRole,
    ) -> Self {
        Self {
            id: UserId::generate(),
            email,
            username,
            password_hash,
            role,
            profile: UserProfile::default(),
        }
    }

    pub fn from_persistence(
        id: UserId,
        email: Email,
        username: Username,
        password_hash: PasswordHash,
        role: UserRole,
        profile: UserProfile,
    ) -> Self {
        Self {
            id,
            email,
            username,
            password_hash,
            role,
            profile,
        }
    }

    pub fn update_password(&mut self, new_hash: PasswordHash) {
        self.password_hash = new_hash;
    }

    pub fn update_profile(&mut self, profile: UserProfile) {
        self.profile = profile;
    }

    pub fn email(&self) -> &Email {
        &self.email
    }
    pub fn username(&self) -> &Username {
        &self.username
    }
    pub fn id(&self) -> &UserId {
        &self.id
    }
    pub fn password_hash(&self) -> &PasswordHash {
        &self.password_hash
    }
    pub fn role(&self) -> &UserRole {
        &self.role
    }
    pub fn display_name(&self) -> Option<&str> {
        self.profile.display_name.as_deref()
    }
    pub fn bio(&self) -> Option<&str> {
        self.profile.bio.as_deref()
    }
    pub fn avatar_path(&self) -> Option<&str> {
        self.profile.avatar_path.as_deref()
    }
    pub fn banner_path(&self) -> Option<&str> {
        self.profile.banner_path.as_deref()
    }
    pub fn also_known_as(&self) -> Option<&str> {
        self.profile.also_known_as.as_deref()
    }
    pub fn profile_fields(&self) -> &[ProfileField] {
        &self.profile.profile_fields
    }
}

#[derive(Clone, Debug)]
pub struct FeedEntry {
    entry: DiaryEntry,
    user_email: String,
}

impl FeedEntry {
    pub fn new(entry: DiaryEntry, user_email: String) -> Self {
        Self { entry, user_email }
    }
    pub fn movie(&self) -> &Movie {
        self.entry.movie()
    }
    pub fn review(&self) -> &Review {
        self.entry.review()
    }
    pub fn user_email(&self) -> &str {
        &self.user_email
    }
    pub fn user_display_name(&self) -> &str {
        self.user_email
            .split('@')
            .next()
            .unwrap_or(&self.user_email)
    }
}

#[derive(Clone, Debug)]
pub struct UserSummary {
    pub user_id: UserId,
    email: Email,
    pub total_movies: i64,
    pub avg_rating: Option<f64>,
    pub avatar_path: Option<String>,
}

impl UserSummary {
    pub fn new(
        user_id: UserId,
        email: Email,
        total_movies: i64,
        avg_rating: Option<f64>,
        avatar_path: Option<String>,
    ) -> Self {
        Self {
            user_id,
            email,
            total_movies,
            avg_rating,
            avatar_path,
        }
    }
    pub fn email(&self) -> &str {
        self.email.value()
    }
}

#[derive(Clone, Debug)]
pub struct UserStats {
    pub total_movies: i64,
    pub avg_rating: Option<f64>,
    pub favorite_director: Option<String>,
    pub most_active_month: Option<String>,
}

#[derive(Clone, Debug)]
pub struct MonthActivity {
    pub year_month: String,
    pub month_label: String,
    pub count: i64,
    pub entries: Vec<DiaryEntry>,
}

#[derive(Clone, Debug)]
pub struct MonthlyRating {
    pub year_month: String,
    pub month_label: String,
    pub avg_rating: f64,
    pub count: i64,
}

#[derive(Clone, Debug)]
pub struct DirectorStat {
    pub director: String,
    pub count: i64,
}

#[derive(Clone, Debug)]
pub struct UserTrends {
    pub monthly_ratings: Vec<MonthlyRating>,
    pub top_directors: Vec<DirectorStat>,
    pub max_director_count: i64,
}

pub enum ExportFormat {
    Csv,
    Json,
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

// ── Movie enrichment ───────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct Genre {
    pub tmdb_id: u32,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct Keyword {
    pub tmdb_id: u32,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct CastMember {
    pub tmdb_person_id: u64,
    pub name: String,
    pub character: String,
    pub billing_order: u32,
    pub profile_path: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CrewMember {
    pub tmdb_person_id: u64,
    pub name: String,
    pub job: String,
    pub department: String,
    pub profile_path: Option<String>,
}

#[derive(Clone, Debug)]
pub struct MovieProfile {
    pub movie_id: MovieId,
    pub tmdb_id: u64,
    pub imdb_id: Option<String>,
    pub overview: Option<String>,
    pub tagline: Option<String>,
    pub runtime_minutes: Option<u32>,
    pub budget_usd: Option<i64>,
    pub revenue_usd: Option<i64>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<u32>,
    pub original_language: Option<String>,
    pub collection_name: Option<String>,
    pub genres: Vec<Genre>,
    pub keywords: Vec<Keyword>,
    pub cast: Vec<CastMember>,
    pub crew: Vec<CrewMember>,
    pub enriched_at: DateTime<Utc>,
}
