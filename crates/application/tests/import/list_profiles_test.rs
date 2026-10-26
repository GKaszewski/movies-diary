use std::sync::Arc;

use domain::testing::InMemoryStore;
use domain::value_objects::UserId;
use uuid::Uuid;

use application::import::{deps::ListImportProfilesDeps, list_profiles};

#[tokio::test]
async fn returns_empty_when_no_profiles() {
    let profiles = InMemoryStore::new();
    let deps = ListImportProfilesDeps {
        import_profile: Arc::clone(&profiles) as _,
    };

    let user_id = UserId::from_uuid(Uuid::new_v4());
    let result = list_profiles::execute(&deps, &user_id).await.unwrap();

    assert!(result.is_empty());
}
