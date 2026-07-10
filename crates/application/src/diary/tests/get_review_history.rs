use std::sync::Arc;

use domain::{
    models::Movie,
    ports::DiaryQuery,
    services::review_history::Trend,
    value_objects::{MovieTitle, ReleaseYear},
};

use crate::{diary::get_review_history, diary::queries::GetReviewHistoryQuery};

#[tokio::test]
async fn returns_empty_history() {
    let movie = Movie::new(
        None,
        MovieTitle::new("Test".into()).unwrap(),
        ReleaseYear::new(2024).unwrap(),
        None,
        None,
    );
    let movie_id = movie.id().value();

    let diary = domain::testing::FakeDiaryQuery::new();
    diary.seed_history(movie, vec![]);
    let diary: Arc<dyn DiaryQuery> = diary;

    let (history, trend) = get_review_history::execute(&diary, GetReviewHistoryQuery { movie_id })
        .await
        .unwrap();

    assert!(history.viewings().is_empty());
    assert_eq!(trend, Trend::Neutral);
}
