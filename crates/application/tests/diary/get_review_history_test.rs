use domain::testing::test_movie_with_year;
use std::sync::Arc;

use domain::{ports::DiaryQuery, services::review_history::Trend};

use application::{
    diary::deps::GetReviewHistoryDeps, diary::get_review_history,
    diary::queries::GetReviewHistoryQuery,
};

#[tokio::test]
async fn returns_empty_history() {
    let movie = test_movie_with_year("Test", 2024);
    let movie_id = movie.id().value();

    let diary = domain::testing::FakeDiaryQuery::new();
    diary.seed_history(movie, vec![]);
    let diary: Arc<dyn DiaryQuery> = diary;
    let deps = GetReviewHistoryDeps { diary };

    let (history, trend) = get_review_history::execute(&deps, GetReviewHistoryQuery { movie_id })
        .await
        .unwrap();

    assert!(history.viewings().is_empty());
    assert_eq!(trend, Trend::Neutral);
}
