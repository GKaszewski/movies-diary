use crate::integrations::cleanup;
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn returns_zero_when_nothing_to_clean() {
    let ctx = TestContextBuilder::new().build();

    let count = cleanup::execute(&ctx).await.unwrap();

    assert_eq!(count, 0);
}
