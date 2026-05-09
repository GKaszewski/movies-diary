use crate::{context::AppContext, queries::GetActivityFeedQuery};
use domain::{
    errors::DomainError,
    models::{
        FeedEntry,
        collections::{PageParams, Paginated},
    },
};

pub async fn execute(
    ctx: &AppContext,
    query: GetActivityFeedQuery,
) -> Result<Paginated<FeedEntry>, DomainError> {
    let page = PageParams::new(Some(query.limit), Some(query.offset))?;
    ctx.diary_repository
        .query_activity_feed_filtered(
            &page,
            &query.sort_by,
            query.search.as_deref(),
            query.following.as_ref(),
        )
        .await
}
