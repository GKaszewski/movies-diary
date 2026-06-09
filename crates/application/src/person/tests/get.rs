use domain::models::PersonId;
use uuid::Uuid;

use crate::person::get;
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn returns_none_for_unknown_person() {
    let ctx = TestContextBuilder::new().build();

    let result = get::execute(&ctx, PersonId::from_uuid(Uuid::new_v4()))
        .await
        .unwrap();

    assert!(result.is_none());
}
