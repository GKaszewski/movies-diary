use domain::{
    errors::DomainError,
    models::{
        Movie, MovieStats, MovieSummary, PersistedReview, Review, ReviewSource, UserSummary,
        WatchlistEntry, WatchlistWithMovie,
    },
    value_objects::{
        Comment, Email, ExternalMetadataId, MovieId, MovieTitle, PosterPath, Rating, ReleaseYear,
        ReviewId, UserId, Username, WatchlistEntryId,
    },
};

/// Map a [`sqlx::Error`] to a [`DomainError::InfrastructureError`], logging the
/// underlying database error at `error` level.
pub fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);
    DomainError::InfrastructureError("Database operation failed".into())
}

/// Parse a string as a UUID, returning a [`DomainError`] on failure.
pub fn parse_uuid(s: &str) -> Result<uuid::Uuid, DomainError> {
    uuid::Uuid::parse_str(s)
        .map_err(|e| DomainError::InfrastructureError(format!("Invalid UUID '{}': {}", s, e)))
}

/// Parse a `%Y-%m-%d %H:%M:%S` string into a [`chrono::NaiveDateTime`].
pub fn parse_datetime(s: &str) -> Result<chrono::NaiveDateTime, DomainError> {
    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .map_err(|e| DomainError::InfrastructureError(format!("Invalid datetime '{}': {}", s, e)))
}

/// Format a [`chrono::NaiveDateTime`] as `%Y-%m-%d %H:%M:%S`.
pub fn datetime_to_str(dt: &chrono::NaiveDateTime) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Convert a `YYYY-MM` string into a human-readable label like `Jan '24`.
pub fn format_year_month(ym: &str) -> String {
    let parts: Vec<&str> = ym.splitn(2, '-').collect();
    if parts.len() != 2 {
        return ym.to_string();
    }
    let year = parts[0].get(2..).unwrap_or(parts[0]);
    let month = match parts[1] {
        "01" => "Jan",
        "02" => "Feb",
        "03" => "Mar",
        "04" => "Apr",
        "05" => "May",
        "06" => "Jun",
        "07" => "Jul",
        "08" => "Aug",
        "09" => "Sep",
        "10" => "Oct",
        "11" => "Nov",
        "12" => "Dec",
        _ => parts[1],
    };
    format!("{} '{}", month, year)
}

// ---------------------------------------------------------------------------
// Shared row-to-domain conversion functions
//
// Each database adapter keeps its own `FromRow` structs (sqlite vs postgres
// derive different impls) but the conversion from parsed row fields into
// domain types is identical.  These functions capture that shared logic so
// each adapter's `into_domain()` becomes a one-liner delegation.
// ---------------------------------------------------------------------------

/// Convert raw movie row fields into a [`Movie`] domain object.
pub fn movie_row_to_domain(
    id: String,
    external_metadata_id: Option<String>,
    title: String,
    release_year: i64,
    director: Option<String>,
    poster_path: Option<String>,
) -> Result<Movie, DomainError> {
    let id = MovieId::from_uuid(parse_uuid(&id)?);
    let external_metadata_id = external_metadata_id
        .map(ExternalMetadataId::new)
        .transpose()?;
    let title = MovieTitle::new(title)?;
    let release_year = ReleaseYear::new(release_year as u16)?;
    let poster_path = poster_path.map(PosterPath::new).transpose()?;
    Ok(Movie::from_persistence(
        id,
        external_metadata_id,
        title,
        release_year,
        director,
        poster_path,
    ))
}

/// Convert raw review row fields into a [`Review`] domain object.
#[allow(clippy::too_many_arguments)]
pub fn review_row_to_domain(
    id: String,
    movie_id: String,
    user_id: String,
    rating: i64,
    comment: Option<String>,
    watched_at: String,
    created_at: String,
    remote_actor_url: Option<String>,
    watch_medium: Option<String>,
) -> Result<Review, DomainError> {
    let id = ReviewId::from_uuid(parse_uuid(&id)?);
    let movie_id = MovieId::from_uuid(parse_uuid(&movie_id)?);
    let user_id = UserId::from_uuid(parse_uuid(&user_id)?);
    let rating = Rating::new(rating as u8)?;
    let comment = comment.map(Comment::new).transpose()?;
    let watched_at = parse_datetime(&watched_at)?;
    let created_at = parse_datetime(&created_at)?;
    let source = match remote_actor_url {
        None => ReviewSource::Local,
        Some(url) => ReviewSource::Remote { actor_url: url },
    };
    let watch_medium = watch_medium.map(|s| s.parse()).transpose()?;
    Ok(Review::from_persistence(PersistedReview {
        id,
        movie_id,
        user_id,
        rating,
        comment,
        watched_at,
        created_at,
        source,
        watch_medium,
    }))
}

/// Assemble a [`MovieSummary`] from an already-converted [`Movie`] and extra
/// metadata fields.  The caller is responsible for converting genres into a
/// `Vec<String>` (sqlite splits a comma-separated string, postgres receives a
/// `Vec` directly).
pub fn movie_summary_to_domain(
    movie: Movie,
    genres: Vec<String>,
    runtime_minutes: Option<i64>,
    original_language: Option<String>,
    overview: Option<String>,
    collection_name: Option<String>,
) -> MovieSummary {
    MovieSummary {
        movie,
        genres,
        runtime_minutes: runtime_minutes.map(|v| v as u32),
        original_language,
        overview,
        collection_name,
    }
}

/// Convert raw aggregate stats into a [`MovieStats`] domain object.
pub fn movie_stats_to_domain(
    total_count: i64,
    avg_rating: Option<f64>,
    federated_count: i64,
    rating_histogram: [i64; 5],
) -> MovieStats {
    MovieStats {
        total_count: total_count as u64,
        avg_rating,
        federated_count: federated_count as u64,
        rating_histogram: [
            rating_histogram[0] as u64,
            rating_histogram[1] as u64,
            rating_histogram[2] as u64,
            rating_histogram[3] as u64,
            rating_histogram[4] as u64,
        ],
    }
}

/// Convert raw user summary row fields into a [`UserSummary`] domain object.
#[allow(clippy::too_many_arguments)]
pub fn user_summary_to_domain(
    id: String,
    email: String,
    username: String,
    display_name: Option<String>,
    total_movies: i64,
    avg_rating: Option<f64>,
    avatar_path: Option<String>,
) -> Result<UserSummary, DomainError> {
    Ok(UserSummary::new(
        UserId::from_uuid(parse_uuid(&id)?),
        Email::new(email)?,
        Username::new(username)?,
        display_name,
        total_movies,
        avg_rating,
        avatar_path,
    ))
}

/// Convert raw watchlist entry fields into a [`WatchlistEntry`] domain object.
pub fn watchlist_entry_to_domain(
    id: String,
    user_id: String,
    movie_id: String,
    added_at: String,
) -> Result<WatchlistEntry, DomainError> {
    Ok(WatchlistEntry {
        id: WatchlistEntryId::from_uuid(parse_uuid(&id)?),
        user_id: UserId::from_uuid(parse_uuid(&user_id)?),
        movie_id: MovieId::from_uuid(parse_uuid(&movie_id)?),
        added_at: parse_datetime(&added_at)?,
    })
}

/// Convert raw watchlist+movie row fields into a [`WatchlistWithMovie`].
///
/// Takes the watchlist entry fields and a pre-converted [`Movie`].
pub fn watchlist_with_movie_to_domain(entry: WatchlistEntry, movie: Movie) -> WatchlistWithMovie {
    WatchlistWithMovie { entry, movie }
}
