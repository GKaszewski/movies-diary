use crate::html::helpers::build_page_context;
use crate::html::templates::MovieDetailTemplate;
use crate::{
    extractors::OptionalCookieUser, html::csrf::CsrfToken, html::render::render_page,
    state::AppState,
};
use application::{
    diary::{get_movie_social_page, queries::GetMovieSocialPageQuery},
    watchlist::{is_on as is_on_watchlist, queries::IsOnWatchlistQuery},
};
use axum::{
    extract::{Extension, Path, Query, State},
    response::IntoResponse,
};

pub async fn get_movie_detail_html(
    OptionalCookieUser(user_id): OptionalCookieUser,
    State(state): State<AppState>,
    Path(movie_id): Path<uuid::Uuid>,
    Query(params): Query<api_types::PaginationQueryParams>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let ctx = build_page_context(&state, user_id.clone(), csrf.0).await;
    let limit = params.limit.unwrap_or(20);
    let offset = params.offset.unwrap_or(0);

    match get_movie_social_page::execute(
        &state.app_ctx.deps.diary.get_movie_social_page,
        GetMovieSocialPageQuery {
            movie_id,
            limit,
            offset,
        },
    )
    .await
    {
        Err(e) => crate::errors::domain_error_response(e),
        Ok(result) => {
            let histogram_max = result
                .stats
                .rating_histogram
                .iter()
                .copied()
                .max()
                .unwrap_or(1);
            let has_more =
                result.reviews.offset + result.reviews.limit < result.reviews.total_count as u32;
            let on_watchlist = match &user_id {
                Some(uid) => is_on_watchlist::execute(
                    &state.app_ctx.deps.watchlist.is_on_watchlist,
                    IsOnWatchlistQuery {
                        user_id: uid.value(),
                        movie_id,
                    },
                )
                .await
                .unwrap_or(false),
                None => false,
            };
            let current_offset = result.reviews.offset;
            let reviews_limit = result.reviews.limit;
            render_page(MovieDetailTemplate {
                ctx: &ctx,
                movie: &result.movie,
                stats: &result.stats,
                profile: result.profile.as_ref(),
                reviews: result.reviews.items.as_slice(),
                on_watchlist,
                current_offset,
                has_more,
                limit: reviews_limit,
                histogram_max,
            })
            .into_response()
        }
    }
}
