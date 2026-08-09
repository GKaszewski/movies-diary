use std::sync::Arc;

use domain::testing::InMemoryImportProfileRepository;
use uuid::Uuid;

use crate::import::{
    commands::DeleteImportProfileCommand, delete_profile, deps::DeleteImportProfileDeps,
};

#[tokio::test]
async fn fails_when_profile_not_found() {
    let profiles = InMemoryImportProfileRepository::new();
    let deps = DeleteImportProfileDeps {
        import_profile: Arc::clone(&profiles) as _,
    };

    let result = delete_profile::execute(
        &deps,
        DeleteImportProfileCommand {
            user_id: Uuid::new_v4(),
            profile_id: Uuid::new_v4(),
        },
    )
    .await;

    assert!(result.is_err());
}
