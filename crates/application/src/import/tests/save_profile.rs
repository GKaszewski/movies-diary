use std::sync::Arc;

use domain::models::ImportSession;
use domain::ports::ImportSessionRepository;
use domain::testing::{InMemoryImportProfileRepository, InMemoryImportSessionRepository};
use domain::value_objects::UserId;
use uuid::Uuid;

use crate::import::deps::SaveProfileDeps;
use crate::import::{commands::SaveImportProfileCommand, save_profile};

#[tokio::test]
async fn fails_when_session_not_found() {
    let sessions = InMemoryImportSessionRepository::new();
    let profiles = InMemoryImportProfileRepository::new();

    let deps = SaveProfileDeps {
        import_session: Arc::clone(&sessions) as _,
        import_profile: Arc::clone(&profiles) as _,
    };

    let result = save_profile::execute(
        &deps,
        SaveImportProfileCommand {
            user_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            name: "my profile".into(),
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn saves_profile_from_session() {
    let sessions = InMemoryImportSessionRepository::new();
    let profiles = InMemoryImportProfileRepository::new();
    let user_id = Uuid::new_v4();

    let mut session = ImportSession::new(UserId::from_uuid(user_id));
    let sid = session.id.clone();
    session.field_mappings = Some(vec![]);
    sessions.create(&session).await.unwrap();

    let deps = SaveProfileDeps {
        import_session: Arc::clone(&sessions) as _,
        import_profile: Arc::clone(&profiles) as _,
    };

    let result = save_profile::execute(
        &deps,
        SaveImportProfileCommand {
            user_id,
            session_id: sid.value(),
            name: "my profile".into(),
        },
    )
    .await;

    assert!(result.is_ok());
}
