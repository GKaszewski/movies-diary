use domain::{
    errors::DomainError,
    models::{RemoteWatchlistEntry, WatchlistWithMovie, collections::Paginated},
    value_objects::UserId,
};
use uuid::Uuid;

use crate::watchlist::deps::{GetWatchlistDeps, GetWatchlistForOwnerDeps};
use crate::watchlist::get;
use crate::watchlist::queries::GetWatchlistQuery;

pub enum WatchlistView {
    Local(Paginated<WatchlistWithMovie>),
    Remote(Vec<RemoteWatchlistEntry>),
}

pub async fn execute(
    deps: &GetWatchlistForOwnerDeps,
    owner_id: Uuid,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<WatchlistView, DomainError> {
    let user_id = UserId::from_uuid(owner_id);

    let is_local = deps
        .user
        .find_by_id(&user_id)
        .await
        .map(|u| u.is_some())
        .unwrap_or(false);

    if is_local {
        let get_deps = GetWatchlistDeps {
            watchlist: deps.watchlist.clone(),
        };
        let paginated = get::execute(
            &get_deps,
            GetWatchlistQuery {
                user_id: owner_id,
                limit,
                offset,
            },
        )
        .await?;
        Ok(WatchlistView::Local(paginated))
    } else {
        let entries = deps
            .remote_watchlist
            .get_by_derived_uuid(owner_id)
            .await
            .unwrap_or_default();
        Ok(WatchlistView::Remote(entries))
    }
}
