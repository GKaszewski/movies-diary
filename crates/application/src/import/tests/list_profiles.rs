use std::sync::Arc;

use domain::testing::InMemoryImportProfileRepository;
use domain::value_objects::UserId;
use uuid::Uuid;

use crate::import::list_profiles;
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn returns_empty_when_no_profiles() {
    let profiles = InMemoryImportProfileRepository::new();
    let ctx = TestContextBuilder::new()
        .with_import_profiles(Arc::clone(&profiles) as _)
        .build();

    let user_id = UserId::from_uuid(Uuid::new_v4());
    let result = list_profiles::execute(&ctx, &user_id).await.unwrap();

    assert!(result.is_empty());
}
