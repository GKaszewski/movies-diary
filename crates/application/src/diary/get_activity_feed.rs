use crate::{context::AppContext, diary::queries::GetActivityFeedQuery};
use domain::{
    errors::DomainError,
    models::{
        FeedEntry,
        collections::{PageParams, Paginated},
    },
    ports::FollowingFilter,
};

pub async fn execute(
    ctx: &AppContext,
    query: GetActivityFeedQuery,
) -> Result<Paginated<FeedEntry>, DomainError> {
    let page = PageParams::new(Some(query.limit), Some(query.offset))?;

    let following = build_following_filter(ctx, &query).await;

    ctx.repos
        .diary
        .query_activity_feed_filtered(
            &page,
            &query.sort_by,
            query.search.as_deref(),
            following.as_ref(),
        )
        .await
}

async fn build_following_filter(
    _ctx: &AppContext,
    query: &GetActivityFeedQuery,
) -> Option<FollowingFilter> {
    #[cfg(not(feature = "federation"))]
    {
        let _ = query;
        return None;
    }
    #[cfg(feature = "federation")]
    {
        if !query.filter_following {
            return None;
        }
        let viewer_id = match query.viewer_user_id {
            Some(id) => id,
            None => return None,
        };
        let urls = _ctx
            .repos
            .social_query
            .get_accepted_following_urls(viewer_id)
            .await
            .unwrap_or_default();
        let base_url = &_ctx.config.base_url;
        let mut local_ids = vec![viewer_id];
        let mut remote_urls = Vec::new();
        for url in urls {
            if let Some(suffix) = url.strip_prefix(&format!("{}/users/", base_url))
                && let Ok(parsed_id) = uuid::Uuid::parse_str(suffix)
            {
                local_ids.push(parsed_id);
                continue;
            }
            remote_urls.push(url);
        }
        Some(FollowingFilter {
            local_user_ids: local_ids,
            remote_actor_urls: remote_urls,
        })
    }
}
