use domain::{errors::DomainError, value_objects::UserId};

use crate::{
    context::AppContext, ports::WatchlistDisplayEntry, watchlist::queries::GetWatchlistQuery,
};

pub struct WatchlistPageResult {
    pub display_entries: Vec<WatchlistDisplayEntry>,
    pub has_more: bool,
    pub current_offset: u32,
    pub limit: u32,
}

pub async fn execute(
    ctx: &AppContext,
    query: GetWatchlistQuery,
    is_owner: bool,
) -> Result<WatchlistPageResult, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);
    let is_local = ctx.repos.user.find_by_id(&user_id).await?.is_some();

    if is_local {
        let page = crate::watchlist::get::execute(ctx, query).await?;
        let has_more = page.offset + page.limit < page.total_count as u32;
        let display_entries = page
            .items
            .iter()
            .map(|w| {
                let remove_url = if is_owner {
                    Some(format!("/watchlist/{}/remove", w.movie.id().value()))
                } else {
                    None
                };
                WatchlistDisplayEntry {
                    poster_url: w
                        .movie
                        .poster_path()
                        .map(|p| format!("/images/{}", p.value())),
                    movie_title: w.movie.title().value().to_string(),
                    release_year: w.movie.release_year().value(),
                    movie_url: Some(format!("/movies/{}", w.movie.id().value())),
                    added_at: w.entry.added_at.format("%b %-d, %Y").to_string(),
                    remove_url,
                }
            })
            .collect();
        Ok(WatchlistPageResult {
            display_entries,
            has_more,
            current_offset: page.offset,
            limit: page.limit,
        })
    } else {
        load_remote_watchlist(ctx, query.user_id).await
    }
}

async fn load_remote_watchlist(
    ctx: &AppContext,
    user_id: uuid::Uuid,
) -> Result<WatchlistPageResult, DomainError> {
    let remote_entries = ctx
        .repos
        .remote_watchlist
        .get_by_derived_uuid(user_id)
        .await
        .unwrap_or_default();
    let len = remote_entries.len() as u32;
    let display_entries = remote_entries
        .into_iter()
        .map(|e| WatchlistDisplayEntry {
            poster_url: e.poster_url,
            movie_title: e.movie_title,
            release_year: e.release_year,
            movie_url: None,
            added_at: e.added_at.format("%b %-d, %Y").to_string(),
            remove_url: None,
        })
        .collect();
    Ok(WatchlistPageResult {
        display_entries,
        has_more: false,
        current_offset: 0,
        limit: len,
    })
}
