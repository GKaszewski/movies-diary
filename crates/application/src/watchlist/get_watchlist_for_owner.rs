use domain::{
    errors::DomainError,
    models::{RemoteWatchlistEntry, WatchlistWithMovie, collections::Paginated},
    value_objects::UserId,
};
use uuid::Uuid;

use crate::watchlist::deps::{GetWatchlistDeps, GetWatchlistForOwnerDeps};
use crate::watchlist::get;
use crate::watchlist::queries::GetWatchlistQuery;

/// Which data source answered a watchlist page request. The former handler
/// decided this by probing for a local user row — that decision now lives here,
/// not in `handlers/watchlist.rs`.
pub enum WatchlistView {
    Local(Paginated<WatchlistWithMovie>),
    Remote(Vec<RemoteWatchlistEntry>),
}

/// `limit`/`offset` are taken raw, not as a pre-built `PageParams`, on purpose:
/// they're only validated on the local branch, by delegating to
/// `watchlist::get::execute` (whose `PageParams::new` call is where the validation
/// actually happens). The remote branch ignores them entirely, unconditionally
/// fetching the full federated watchlist. The delegation call below must stay
/// inside the `if is_local` branch — calling it before the local/remote decision
/// would turn an out-of-range `limit` into a 400 on a remote owner's page, which
/// never happened before and must not start happening now.
pub async fn execute(
    deps: &GetWatchlistForOwnerDeps,
    owner_id: Uuid,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<WatchlistView, DomainError> {
    let user_id = UserId::from_uuid(owner_id);

    // Matches the deleted handler's `.map(|u| u.is_some()).unwrap_or(false)`
    // exactly: a lookup error is treated the same as "no local row", falling
    // through to the remote arm rather than propagating.
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
        // Matches the deleted handler's `.unwrap_or_default()`: a federation
        // lookup error yields an empty list, not a propagated error.
        let entries = deps
            .remote_watchlist
            .get_by_derived_uuid(owner_id)
            .await
            .unwrap_or_default();
        Ok(WatchlistView::Remote(entries))
    }
}

#[cfg(test)]
#[path = "tests/get_watchlist_for_owner.rs"]
mod tests;
