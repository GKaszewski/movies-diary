use std::sync::Arc;

use domain::testing::InMemoryImportProfileRepository;
use uuid::Uuid;

use crate::import::{commands::DeleteImportProfileCommand, delete_profile};
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn fails_when_profile_not_found() {
    let profiles = InMemoryImportProfileRepository::new();
    let ctx = TestContextBuilder::new()
        .with_import_profiles(Arc::clone(&profiles) as _)
        .build();

    let result = delete_profile::execute(
        &ctx,
        DeleteImportProfileCommand {
            user_id: Uuid::new_v4(),
            profile_id: Uuid::new_v4(),
        },
    )
    .await;

    assert!(result.is_err());
}
