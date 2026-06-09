use domain::models::PersonId;
use uuid::Uuid;

use crate::person::get_credits;
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn returns_empty_credits() {
    let ctx = TestContextBuilder::new().build();

    let result = get_credits::execute(&ctx, PersonId::from_uuid(Uuid::new_v4()))
        .await
        .unwrap();

    assert!(result.cast.is_empty());
    assert!(result.crew.is_empty());
}
