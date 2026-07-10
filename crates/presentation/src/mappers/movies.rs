use api_types::{DiaryEntryDto, MovieDto, ReviewDto};
use domain::models::{DiaryEntry, Movie, MovieSummary, Review};

pub fn movie_to_dto(movie: &Movie) -> MovieDto {
    MovieDto {
        id: movie.id().value(),
        title: movie.title().value().to_string(),
        release_year: movie.release_year().value(),
        director: movie.director().map(|d| d.to_string()),
        poster_path: movie.poster_path().map(|p| p.value().to_string()),
        genres: vec![],
        runtime_minutes: None,
        original_language: None,
        overview: None,
        collection_name: None,
    }
}

pub fn summary_to_dto(summary: &MovieSummary) -> MovieDto {
    MovieDto {
        id: summary.movie.id().value(),
        title: summary.movie.title().value().to_string(),
        release_year: summary.movie.release_year().value(),
        director: summary.movie.director().map(|d| d.to_string()),
        poster_path: summary.movie.poster_path().map(|p| p.value().to_string()),
        genres: summary.genres.clone(),
        runtime_minutes: summary.runtime_minutes,
        original_language: summary.original_language.clone(),
        overview: summary.overview.clone(),
        collection_name: summary.collection_name.clone(),
    }
}

pub fn review_to_dto(review: &Review) -> ReviewDto {
    ReviewDto {
        id: review.id().value(),
        rating: review.rating().value(),
        comment: review.comment().map(|c| c.value().to_string()),
        watched_at: domain::value_objects::format_watched_at(review.watched_at()),
        watch_medium: review.watch_medium().copied(),
    }
}

pub fn entry_to_dto(entry: &DiaryEntry) -> DiaryEntryDto {
    DiaryEntryDto {
        movie: movie_to_dto(entry.movie()),
        review: review_to_dto(entry.review()),
    }
}
