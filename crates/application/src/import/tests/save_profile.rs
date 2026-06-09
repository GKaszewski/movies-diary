use std::sync::Arc;

use chrono::Utc;
use domain::models::ImportSession;
use domain::ports::ImportSessionRepository;
use domain::testing::InMemoryImportSessionRepository;
use domain::value_objects::{ImportSessionId, UserId};
use uuid::Uuid;

use crate::import::{commands::SaveImportProfileCommand, save_profile};
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn fails_when_session_not_found() {
    let sessions = InMemoryImportSessionRepository::new();
    let ctx = TestContextBuilder::new()
        .with_import_sessions(Arc::clone(&sessions) as _)
        .build();

    let result = save_profile::execute(
        &ctx,
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
    let user_id = Uuid::new_v4();
    let sid = ImportSessionId::generate();

    let mut session = ImportSession::new(
        sid.clone(),
        UserId::from_uuid(user_id),
        Utc::now().naive_utc(),
    );
    session.field_mappings = Some(vec![]);
    sessions.create(&session).await.unwrap();

    let ctx = TestContextBuilder::new()
        .with_import_sessions(Arc::clone(&sessions) as _)
        .build();

    let result = save_profile::execute(
        &ctx,
        SaveImportProfileCommand {
            user_id,
            session_id: sid.value(),
            name: "my profile".into(),
        },
    )
    .await;

    assert!(result.is_ok());
}
