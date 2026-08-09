//! Shared, pure helpers for turning a `GetUserProfileQuery`'s sort/paging/search
//! fields into a `DiaryFilter`. Used by both halves of the former `get_profile`
//! split (`get_local_profile`, `get_federated_profile_stats`) so the query-shaping
//! logic has exactly one definition.

use domain::{
    errors::DomainError,
    models::{DiaryFilter, FeedSortBy, ReviewSortBy, collections::PageParams},
    value_objects::UserId,
};

pub(super) fn feed_sort_to_direction(sort_by: FeedSortBy) -> ReviewSortBy {
    match sort_by {
        FeedSortBy::Date => ReviewSortBy::Descending,
        FeedSortBy::DateAsc => ReviewSortBy::Ascending,
        FeedSortBy::Rating => ReviewSortBy::ByRatingDesc,
        FeedSortBy::RatingAsc => ReviewSortBy::ByRatingAsc,
    }
}

pub(super) fn paged_user_filter(
    user_id: UserId,
    sort_by: ReviewSortBy,
    limit: Option<u32>,
    offset: Option<u32>,
    search: Option<String>,
    include_remote: bool,
) -> Result<DiaryFilter, DomainError> {
    let page = PageParams::new(limit, offset)?;
    Ok(DiaryFilter {
        sort_by,
        page,
        movie_id: None,
        user_id: Some(user_id),
        search,
        include_remote,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feed_sort_to_direction_all_variants() {
        assert!(matches!(
            feed_sort_to_direction(FeedSortBy::Date),
            ReviewSortBy::Descending
        ));
        assert!(matches!(
            feed_sort_to_direction(FeedSortBy::DateAsc),
            ReviewSortBy::Ascending
        ));
        assert!(matches!(
            feed_sort_to_direction(FeedSortBy::Rating),
            ReviewSortBy::ByRatingDesc
        ));
        assert!(matches!(
            feed_sort_to_direction(FeedSortBy::RatingAsc),
            ReviewSortBy::ByRatingAsc
        ));
    }

    #[test]
    fn paged_user_filter_builds_correctly() {
        let uid = UserId::from_uuid(uuid::Uuid::new_v4());
        let filter = paged_user_filter(
            uid.clone(),
            ReviewSortBy::Descending,
            Some(20),
            Some(5),
            Some("blade".into()),
            false,
        )
        .unwrap();

        assert_eq!(filter.user_id.unwrap().value(), uid.value());
        assert_eq!(filter.search.as_deref(), Some("blade"));
    }
}
