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
    let page = PageParams::new(query.limit, query.offset)?;
    ctx.diary_repository.query_activity_feed(&page).await
}
