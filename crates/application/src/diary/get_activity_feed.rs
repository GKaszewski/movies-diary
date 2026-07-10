use crate::diary::{deps::GetActivityFeedDeps, queries::GetActivityFeedQuery};
use domain::{
    errors::DomainError,
    models::FollowingFilter,
    models::{
        FeedEntry,
        collections::{PageParams, Paginated},
    },
    value_objects::{SocialIdentity, UserId},
};

pub async fn execute(
    deps: &GetActivityFeedDeps,
    query: GetActivityFeedQuery,
) -> Result<Paginated<FeedEntry>, DomainError> {
    let page = PageParams::new(Some(query.limit), Some(query.offset))?;

    let following = build_following_filter(deps, &query).await;

    deps.diary
        .query_activity_feed_filtered(
            &page,
            &query.sort_by,
            query.search.as_deref(),
            following.as_ref(),
        )
        .await
}

async fn build_following_filter(
    deps: &GetActivityFeedDeps,
    query: &GetActivityFeedQuery,
) -> Option<FollowingFilter> {
    if !query.filter_following {
        return None;
    }
    let viewer_id = query.viewer_user_id?;
    let viewer = UserId::from_uuid(viewer_id);
    let actors = deps
        .social_query
        .get_following(&viewer)
        .await
        .unwrap_or_default();
    if actors.is_empty() {
        return Some(FollowingFilter {
            local_user_ids: vec![viewer_id],
            remote_actor_urls: vec![],
        });
    }
    let mut local_ids = vec![viewer_id];
    let mut remote_urls = Vec::new();
    for actor in actors {
        match actor.identity {
            SocialIdentity::Local(uid) => local_ids.push(uid.value()),
            SocialIdentity::Remote { actor_url } => remote_urls.push(actor_url),
        }
    }
    Some(FollowingFilter {
        local_user_ids: local_ids,
        remote_actor_urls: remote_urls,
    })
}

#[cfg(test)]
#[path = "tests/get_activity_feed.rs"]
mod tests;
