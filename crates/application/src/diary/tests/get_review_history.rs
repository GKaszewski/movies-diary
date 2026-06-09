use domain::{
    models::Movie,
    services::review_history::Trend,
    value_objects::{MovieTitle, ReleaseYear},
};

use crate::{
    diary::get_review_history, diary::queries::GetReviewHistoryQuery,
    test_helpers::TestContextBuilder,
};

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

    let diary = domain::testing::FakeDiaryRepository::new();
    diary.seed_history(movie, vec![]);

    let ctx = TestContextBuilder::new().with_diary(diary as _).build();

    let (history, trend) = get_review_history::execute(&ctx, GetReviewHistoryQuery { movie_id })
        .await
        .unwrap();

    assert!(history.viewings().is_empty());
    assert_eq!(trend, Trend::Neutral);
}
