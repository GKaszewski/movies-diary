use api_types::{AddToWatchlistRequest, WatchlistEntryDto, WatchlistResponse, WatchlistStatusResponse};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::api::get_watchlist_handler,
        crate::handlers::api::post_watchlist_add,
        crate::handlers::api::delete_watchlist_entry,
        crate::handlers::api::get_watchlist_status,
    ),
    components(schemas(
        WatchlistResponse,
        WatchlistEntryDto,
        AddToWatchlistRequest,
        WatchlistStatusResponse,
    ))
)]
pub struct WatchlistDoc;
