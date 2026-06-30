use std::sync::Arc;

use domain::{
    errors::DomainError,
    models::{
        DiaryEntry, DiaryFilter, SortDirection,
        collections::{PageParams, Paginated},
    },
    ports::DiaryRepository,
    value_objects::{MovieId, UserId},
};

use crate::diary::queries::GetDiaryQuery;

pub async fn execute(
    diary: &Arc<dyn DiaryRepository>,
    query: GetDiaryQuery,
) -> Result<Paginated<DiaryEntry>, DomainError> {
    let page = PageParams::new(query.limit, query.offset)?;
    let movie_id = query.movie_id.map(MovieId::from_uuid);
    let user_id = query.user_id.map(UserId::from_uuid);

    let filter = DiaryFilter {
        sort_by: query.sort_by.unwrap_or(SortDirection::Descending),
        page,
        movie_id,
        user_id,
        search: None,
        include_remote: false,
    };

    diary.query_diary(&filter).await
}

#[cfg(test)]
#[path = "tests/get_diary.rs"]
mod tests;
