use std::sync::Arc;

use domain::testing::InMemoryImportProfileRepository;
use uuid::Uuid;

use crate::import::{commands::DeleteImportProfileCommand, delete_profile};

#[tokio::test]
async fn fails_when_profile_not_found() {
    let profiles = InMemoryImportProfileRepository::new();

    let result = delete_profile::execute(
        Arc::clone(&profiles) as _,
        DeleteImportProfileCommand {
            user_id: Uuid::new_v4(),
            profile_id: Uuid::new_v4(),
        },
    )
    .await;

    assert!(result.is_err());
}
