use domain::models::{RemoteWatchlistEntry, WatchlistWithMovie, collections::Paginated};
use template_askama::WatchlistDisplayEntry;

pub struct WatchlistPageResult {
    pub display_entries: Vec<WatchlistDisplayEntry>,
    pub has_more: bool,
    pub current_offset: u32,
    pub limit: u32,
}

pub fn build_watchlist_page(
    page: Paginated<WatchlistWithMovie>,
    is_owner: bool,
) -> WatchlistPageResult {
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
    WatchlistPageResult {
        display_entries,
        has_more,
        current_offset: page.offset,
        limit: page.limit,
    }
}

pub fn build_remote_watchlist_page(entries: Vec<RemoteWatchlistEntry>) -> WatchlistPageResult {
    let len = entries.len() as u32;
    let display_entries = entries
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
    WatchlistPageResult {
        display_entries,
        has_more: false,
        current_offset: 0,
        limit: len,
    }
}
