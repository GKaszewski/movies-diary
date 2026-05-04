pub mod html {
    use axum::{
        extract::{Query, State},
        response::{Html, IntoResponse, Redirect},
        Form,
    };
    use chrono::NaiveDateTime;

    use application::{
        commands::LogReviewCommand,
        queries::GetDiaryQuery,
        use_cases::{get_diary, log_review},
    };
    use domain::{errors::DomainError, models::SortDirection};

    use crate::{
        dtos::{DiaryQueryParams, LogReviewForm},
        errors::ApiError,
        extractors::AuthenticatedUser,
        state::AppState,
    };

    pub async fn get_diary_page(
        State(state): State<AppState>,
        Query(params): Query<DiaryQueryParams>,
    ) -> Result<impl IntoResponse, ApiError> {
        let query = GetDiaryQuery {
            limit: params.limit,
            offset: params.offset,
            sort_by: params.sort_by.as_deref().map(|s| {
                if s == "asc" {
                    SortDirection::Ascending
                } else {
                    SortDirection::Descending
                }
            }),
            movie_id: params.movie_id,
        };

        let page = get_diary::execute(&state.app_ctx, query).await?;
        let html = state
            .html_renderer
            .render_diary_page(&page)
            .map_err(|e| ApiError(DomainError::InfrastructureError(e)))?;

        Ok(Html(html))
    }

    pub async fn post_review(
        State(state): State<AppState>,
        user: AuthenticatedUser,
        Form(form): Form<LogReviewForm>,
    ) -> Result<impl IntoResponse, ApiError> {
        let watched_at = NaiveDateTime::parse_from_str(&form.watched_at, "%Y-%m-%dT%H:%M:%S")
            .map_err(|_| {
                ApiError(DomainError::ValidationError(
                    "Invalid watched_at format, expected YYYY-MM-DDTHH:MM:SS".into(),
                ))
            })?;

        let cmd = LogReviewCommand {
            external_metadata_id: form.external_metadata_id,
            manual_title: form.manual_title,
            manual_release_year: form.manual_release_year,
            manual_director: form.manual_director,
            user_id: user.0.value(),
            rating: form.rating,
            comment: form.comment,
            watched_at,
        };

        log_review::execute(&state.app_ctx, cmd).await?;

        Ok(Redirect::to("/diary"))
    }
}

pub mod rss {
    use axum::{
        extract::State,
        http::header,
        response::IntoResponse,
    };

    use application::{queries::GetDiaryQuery, use_cases::get_diary};
    use domain::{errors::DomainError, models::SortDirection};

    use crate::{errors::ApiError, state::AppState};

    pub async fn get_feed(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
        let query = GetDiaryQuery {
            limit: Some(50),
            offset: Some(0),
            sort_by: Some(SortDirection::Descending),
            movie_id: None,
        };
        let page = get_diary::execute(&state.app_ctx, query).await?;
        let xml = state
            .rss_renderer
            .render_feed(&page.items)
            .map_err(|e| ApiError(DomainError::InfrastructureError(e)))?;
        Ok(([(header::CONTENT_TYPE, "application/rss+xml; charset=utf-8")], xml))
    }
}

pub mod api {
    use axum::{
        Json,
        extract::{Path, Query, State},
        http::StatusCode,
        response::IntoResponse,
    };
    use chrono::NaiveDateTime;
    use uuid::Uuid;

    use application::{
        commands::{LoginCommand, LogReviewCommand, RegisterCommand, SyncPosterCommand},
        queries::{GetDiaryQuery, GetReviewHistoryQuery},
        use_cases::{get_diary, get_review_history, log_review, login as login_uc, register as register_uc, sync_poster},
    };
    use domain::{
        errors::DomainError,
        models::{DiaryEntry, Movie, Review, SortDirection},
        services::review_history::Trend,
        value_objects::MovieId,
    };

    use crate::{
        dtos::{
            DiaryEntryDto, DiaryQueryParams, DiaryResponse, LoginRequest, LoginResponse,
            LogReviewRequest, MovieDto, RegisterRequest, ReviewDto, ReviewHistoryResponse,
        },
        errors::ApiError,
        extractors::AuthenticatedUser,
        state::AppState,
    };

    pub async fn get_diary(
        State(state): State<AppState>,
        Query(params): Query<DiaryQueryParams>,
    ) -> Result<Json<DiaryResponse>, ApiError> {
        let query = GetDiaryQuery {
            limit: params.limit,
            offset: params.offset,
            sort_by: params.sort_by.as_deref().map(|s| {
                if s == "asc" {
                    SortDirection::Ascending
                } else {
                    SortDirection::Descending
                }
            }),
            movie_id: params.movie_id,
        };

        let page = get_diary::execute(&state.app_ctx, query).await?;

        Ok(Json(DiaryResponse {
            items: page.items.iter().map(entry_to_dto).collect(),
            total_count: page.total_count,
            limit: page.limit,
            offset: page.offset,
        }))
    }

    pub async fn get_review_history(
        State(state): State<AppState>,
        Path(movie_id): Path<Uuid>,
    ) -> Result<Json<ReviewHistoryResponse>, ApiError> {
        let (history, trend) = get_review_history::execute(
            &state.app_ctx,
            GetReviewHistoryQuery { movie_id },
        )
        .await?;

        Ok(Json(ReviewHistoryResponse {
            movie: movie_to_dto(history.movie()),
            viewings: history.viewings().iter().map(review_to_dto).collect(),
            trend: match trend {
                Trend::Improved => "improved",
                Trend::Declined => "declined",
                Trend::Neutral => "neutral",
            }
            .to_string(),
        }))
    }

    pub async fn post_review(
        State(state): State<AppState>,
        user: AuthenticatedUser,
        Json(req): Json<LogReviewRequest>,
    ) -> Result<impl IntoResponse, ApiError> {
        let watched_at = NaiveDateTime::parse_from_str(&req.watched_at, "%Y-%m-%dT%H:%M:%S")
            .map_err(|_| {
                ApiError(DomainError::ValidationError(
                    "Invalid watched_at format, expected YYYY-MM-DDTHH:MM:SS".into(),
                ))
            })?;

        let cmd = LogReviewCommand {
            external_metadata_id: req.external_metadata_id,
            manual_title: req.manual_title,
            manual_release_year: req.manual_release_year,
            manual_director: req.manual_director,
            user_id: user.0.value(),
            rating: req.rating,
            comment: req.comment,
            watched_at,
        };

        log_review::execute(&state.app_ctx, cmd).await?;

        Ok(StatusCode::CREATED)
    }

    pub async fn sync_poster(
        State(state): State<AppState>,
        _user: AuthenticatedUser,
        Path(movie_id): Path<Uuid>,
    ) -> Result<impl IntoResponse, ApiError> {
        let movie = state
            .app_ctx
            .repository
            .get_movie_by_id(&MovieId::from_uuid(movie_id))
            .await?
            .ok_or_else(|| ApiError(DomainError::NotFound(format!("Movie {movie_id}"))))?;

        let external_id = movie
            .external_metadata_id()
            .ok_or_else(|| {
                ApiError(DomainError::ValidationError(
                    "Movie has no external metadata ID, cannot sync poster".into(),
                ))
            })?
            .value()
            .to_string();

        sync_poster::execute(
            &state.app_ctx,
            SyncPosterCommand {
                movie_id,
                external_metadata_id: external_id,
            },
        )
        .await?;

        Ok(StatusCode::NO_CONTENT)
    }

    pub async fn login(
        State(state): State<AppState>,
        Json(req): Json<LoginRequest>,
    ) -> Result<Json<LoginResponse>, ApiError> {
        let result = login_uc::execute(&state.app_ctx, LoginCommand {
            email: req.email,
            password: req.password,
        })
        .await?;
        Ok(Json(LoginResponse {
            token: result.token,
            user_id: result.user_id,
            email: result.email,
            expires_at: result.expires_at.to_rfc3339(),
        }))
    }

    pub async fn register(
        State(state): State<AppState>,
        Json(req): Json<RegisterRequest>,
    ) -> Result<StatusCode, ApiError> {
        register_uc::execute(&state.app_ctx, RegisterCommand {
            email: req.email,
            password: req.password,
        })
        .await?;
        Ok(StatusCode::CREATED)
    }

    fn movie_to_dto(movie: &Movie) -> MovieDto {
        MovieDto {
            id: movie.id().value(),
            title: movie.title().value().to_string(),
            release_year: movie.release_year().value(),
            director: movie.director().map(|d| d.to_string()),
            poster_path: movie.poster_path().map(|p| p.value().to_string()),
        }
    }

    fn review_to_dto(review: &Review) -> ReviewDto {
        ReviewDto {
            id: review.id().value(),
            rating: review.rating().value(),
            comment: review.comment().map(|c| c.value().to_string()),
            watched_at: review.watched_at().to_string(),
        }
    }

    fn entry_to_dto(entry: &DiaryEntry) -> DiaryEntryDto {
        DiaryEntryDto {
            movie: movie_to_dto(entry.movie()),
            review: review_to_dto(entry.review()),
        }
    }
}
