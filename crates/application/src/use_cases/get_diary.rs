use domain::{
    errors::DomainError,
    models::{
        DiaryEntry, DiaryFilter, SortDirection,
        collections::{PageParams, Paginated},
    },
    value_objects::MovieId,
};

use crate::{context::AppContext, queries::GetDiaryQuery};

pub async fn execute(
    ctx: &AppContext,
    query: GetDiaryQuery,
) -> Result<Paginated<DiaryEntry>, DomainError> {
    let page = PageParams::new(query.limit, query.offset)?;

    let movie_id = query.movie_id.map(MovieId::from_uuid);

    let filter = DiaryFilter {
        sort_by: query.sort_by.unwrap_or(SortDirection::Descending),
        page,
        movie_id,
    };

    let paginated_results = ctx.repository.query_diary(&filter).await?;

    Ok(paginated_results)
}
