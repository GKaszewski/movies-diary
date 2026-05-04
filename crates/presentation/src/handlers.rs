pub mod html {
    use axum::{
        extract::{Path, Query, State},
        http::{HeaderValue, StatusCode, header::SET_COOKIE},
        response::{Html, IntoResponse, Redirect},
        Form,
    };
    use chrono::{NaiveDateTime, Utc};
    use uuid::Uuid;

    use application::{
        commands::{DeleteReviewCommand, LoginCommand, LogReviewCommand, RegisterCommand},
        ports::{HtmlPageContext, LoginPageData, NewReviewPageData, RegisterPageData},
        queries::GetDiaryQuery,
        use_cases::{delete_review, get_diary, log_review, login as login_uc, register as register_uc},
    };
    use domain::{errors::DomainError, models::SortDirection, value_objects::UserId};

    use crate::{
        dtos::{DiaryQueryParams, ErrorQuery, LoginForm, LogReviewForm, RegisterForm},
        errors::ApiError,
        extractors::{OptionalCookieUser, RequiredCookieUser},
        state::AppState,
    };

    async fn build_page_context(state: &AppState, user_id: Option<UserId>) -> HtmlPageContext {
        let user_email = if let Some(id) = user_id {
            state
                .app_ctx
                .user_repository
                .find_by_id(&id)
                .await
                .ok()
                .flatten()
                .map(|u| u.email().value().to_string())
        } else {
            None
        };
        HtmlPageContext {
            user_email,
            register_enabled: state.app_ctx.config.allow_registration,
        }
    }

    fn encode_error(msg: &str) -> String {
        msg.replace(' ', "+")
            .replace('&', "%26")
            .replace('=', "%3D")
            .replace('"', "%22")
    }

    fn set_cookie_header(token: &str, max_age: i64) -> (axum::http::HeaderName, HeaderValue) {
        let val = format!(
            "token={}; HttpOnly; Path=/; SameSite=Lax; Max-Age={}",
            token, max_age
        );
        (SET_COOKIE, HeaderValue::from_str(&val).expect("valid cookie"))
    }

    pub async fn get_index(
        OptionalCookieUser(user_id): OptionalCookieUser,
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
        let ctx = build_page_context(&state, user_id).await;
        let page = get_diary::execute(&state.app_ctx, query).await?;
        let html = state
            .html_renderer
            .render_diary_page(&page, ctx)
            .map_err(|e| ApiError(DomainError::InfrastructureError(e)))?;
        Ok(Html(html))
    }

    pub async fn get_login_page(
        State(state): State<AppState>,
        Query(params): Query<ErrorQuery>,
    ) -> impl IntoResponse {
        let ctx = HtmlPageContext {
            user_email: None,
            register_enabled: state.app_ctx.config.allow_registration,
        };
        let html = state
            .html_renderer
            .render_login_page(LoginPageData {
                ctx,
                error: params.error.as_deref(),
            })
            .expect("login template failed");
        Html(html)
    }

    pub async fn post_login(
        State(state): State<AppState>,
        Form(form): Form<LoginForm>,
    ) -> impl IntoResponse {
        match login_uc::execute(
            &state.app_ctx,
            LoginCommand {
                email: form.email,
                password: form.password,
            },
        )
        .await
        {
            Ok(result) => {
                let max_age = (result.expires_at - Utc::now()).num_seconds().max(0);
                let cookie = set_cookie_header(&result.token, max_age);
                ([cookie], Redirect::to("/")).into_response()
            }
            Err(_) => Redirect::to("/login?error=Invalid+credentials").into_response(),
        }
    }

    pub async fn get_logout() -> impl IntoResponse {
        let cookie = (
            SET_COOKIE,
            HeaderValue::from_static("token=; HttpOnly; Path=/; SameSite=Lax; Max-Age=0"),
        );
        ([cookie], Redirect::to("/")).into_response()
    }

    pub async fn get_register_page(
        State(state): State<AppState>,
        Query(params): Query<ErrorQuery>,
    ) -> impl IntoResponse {
        if !state.app_ctx.config.allow_registration {
            return Redirect::to("/").into_response();
        }
        let ctx = HtmlPageContext {
            user_email: None,
            register_enabled: true,
        };
        let html = state
            .html_renderer
            .render_register_page(RegisterPageData {
                ctx,
                error: params.error.as_deref(),
            })
            .expect("register template failed");
        Html(html).into_response()
    }

    pub async fn post_register(
        State(state): State<AppState>,
        Form(form): Form<RegisterForm>,
    ) -> impl IntoResponse {
        if !state.app_ctx.config.allow_registration {
            return Redirect::to("/").into_response();
        }
        let email = form.email.clone();
        let password = form.password.clone();
        match register_uc::execute(
            &state.app_ctx,
            RegisterCommand {
                email: form.email,
                password: form.password,
            },
        )
        .await
        {
            Ok(_) => {
                match login_uc::execute(&state.app_ctx, LoginCommand { email, password }).await {
                    Ok(result) => {
                        let max_age = (result.expires_at - Utc::now()).num_seconds().max(0);
                        let cookie = set_cookie_header(&result.token, max_age);
                        ([cookie], Redirect::to("/")).into_response()
                    }
                    Err(_) => Redirect::to("/login").into_response(),
                }
            }
            Err(e) => {
                let msg = encode_error(&e.to_string());
                Redirect::to(&format!("/register?error={}", msg)).into_response()
            }
        }
    }

    pub async fn get_new_review_page(
        RequiredCookieUser(user_id): RequiredCookieUser,
        State(state): State<AppState>,
        Query(params): Query<ErrorQuery>,
    ) -> impl IntoResponse {
        let ctx = build_page_context(&state, Some(user_id)).await;
        let html = state
            .html_renderer
            .render_new_review_page(NewReviewPageData {
                ctx,
                error: params.error.as_deref(),
            })
            .expect("new_review template failed");
        Html(html)
    }

    pub async fn post_review(
        State(state): State<AppState>,
        RequiredCookieUser(user_id): RequiredCookieUser,
        Form(form): Form<LogReviewForm>,
    ) -> impl IntoResponse {
        let watched_at = NaiveDateTime::parse_from_str(&form.watched_at, "%Y-%m-%dT%H:%M:%S")
            .or_else(|_| NaiveDateTime::parse_from_str(&form.watched_at, "%Y-%m-%dT%H:%M"));

        let watched_at = match watched_at {
            Ok(dt) => dt,
            Err(_) => {
                return Redirect::to("/reviews/new?error=Invalid+date+format").into_response()
            }
        };

        let cmd = LogReviewCommand {
            external_metadata_id: form.external_metadata_id.filter(|s| !s.trim().is_empty()),
            manual_title: form.manual_title,
            manual_release_year: form.manual_release_year,
            manual_director: form.manual_director,
            user_id: user_id.value(),
            rating: form.rating,
            comment: form.comment,
            watched_at,
        };

        match log_review::execute(&state.app_ctx, cmd).await {
            Ok(_) => Redirect::to("/").into_response(),
            Err(e) => {
                let msg = encode_error(&e.to_string());
                Redirect::to(&format!("/reviews/new?error={}", msg)).into_response()
            }
        }
    }

    pub async fn post_delete_review(
        State(state): State<AppState>,
        RequiredCookieUser(user_id): RequiredCookieUser,
        Path(review_id): Path<Uuid>,
    ) -> impl IntoResponse {
        let cmd = DeleteReviewCommand {
            review_id,
            requesting_user_id: user_id.value(),
        };
        match delete_review::execute(&state.app_ctx, cmd).await {
            Ok(()) => Redirect::to("/").into_response(),
            Err(DomainError::NotFound(_)) => StatusCode::NOT_FOUND.into_response(),
            Err(DomainError::Unauthorized(_)) => StatusCode::FORBIDDEN.into_response(),
            Err(e) => {
                tracing::error!("delete_review html error: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub mod posters {
    use axum::{
        extract::{Path, State},
        http::{StatusCode, header},
        response::IntoResponse,
    };

    use domain::value_objects::PosterPath;

    use crate::state::AppState;

    pub async fn get_poster(
        State(state): State<AppState>,
        Path(path): Path<String>,
    ) -> impl IntoResponse {
        let poster_path = match PosterPath::new(path) {
            Ok(p) => p,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        };
        match state.app_ctx.poster_storage.get_poster(&poster_path).await {
            Ok(bytes) => {
                let mime = infer::get(&bytes)
                    .map(|t| t.mime_type())
                    .unwrap_or("application/octet-stream");
                ([(header::CONTENT_TYPE, mime)], bytes).into_response()
            }
            Err(_) => StatusCode::NOT_FOUND.into_response(),
        }
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
        commands::{DeleteReviewCommand, LoginCommand, LogReviewCommand, RegisterCommand, SyncPosterCommand},
        queries::{GetDiaryQuery, GetReviewHistoryQuery},
        use_cases::{delete_review, get_diary, get_review_history, log_review, login as login_uc, register as register_uc, sync_poster},
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
            external_metadata_id: req.external_metadata_id.filter(|s| !s.trim().is_empty()),
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

    pub async fn delete_review(
        State(state): State<AppState>,
        AuthenticatedUser(user_id): AuthenticatedUser,
        Path(review_id): Path<Uuid>,
    ) -> impl IntoResponse {
        let cmd = DeleteReviewCommand {
            review_id,
            requesting_user_id: user_id.value(),
        };
        match delete_review::execute(&state.app_ctx, cmd).await {
            Ok(()) => StatusCode::NO_CONTENT.into_response(),
            Err(DomainError::NotFound(_)) => StatusCode::NOT_FOUND.into_response(),
            Err(DomainError::Unauthorized(_)) => StatusCode::FORBIDDEN.into_response(),
            Err(e) => {
                tracing::error!("delete_review error: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
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
