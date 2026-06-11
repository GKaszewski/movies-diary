use std::sync::Arc;

use chrono::Utc;
use domain::models::ImportProfile;
use domain::ports::{ImportProfileRepository, ImportSessionRepository};
use domain::testing::{InMemoryImportProfileRepository, InMemoryImportSessionRepository};
use domain::value_objects::{ImportProfileId, UserId};
use uuid::Uuid;

use crate::import::{apply_profile, commands::ApplyImportProfileCommand};

#[tokio::test]
async fn fails_when_profile_not_found() {
    let profiles = InMemoryImportProfileRepository::new();
    let sessions = InMemoryImportSessionRepository::new();

    let result = apply_profile::execute(
        Arc::clone(&profiles) as _,
        Arc::clone(&sessions) as _,
        ApplyImportProfileCommand {
            user_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            profile_id: Uuid::new_v4(),
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn fails_when_session_not_found() {
    let profiles = InMemoryImportProfileRepository::new();
    let sessions = InMemoryImportSessionRepository::new();
    let user_id = Uuid::new_v4();

    let profile = ImportProfile::new(
        ImportProfileId::generate(),
        UserId::from_uuid(user_id),
        "test".into(),
        vec![],
        Utc::now().naive_utc(),
    );
    let profile_id = profile.id.clone();
    profiles.save(&profile).await.unwrap();

    let result = apply_profile::execute(
        Arc::clone(&profiles) as _,
        Arc::clone(&sessions) as _,
        ApplyImportProfileCommand {
            user_id,
            session_id: Uuid::new_v4(),
            profile_id: profile_id.value(),
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn applies_profile_mappings_to_session() {
    let profiles = InMemoryImportProfileRepository::new();
    let sessions = domain::testing::InMemoryImportSessionRepository::new();
    let user_id = Uuid::new_v4();

    let profile = ImportProfile::new(
        ImportProfileId::generate(),
        UserId::from_uuid(user_id),
        "letterboxd".into(),
        vec![domain::models::FieldMapping {
            source_column: "Name".into(),
            domain_field: domain::models::import::DomainField::Title,
            transform: domain::models::import::Transform::Identity,
        }],
        Utc::now().naive_utc(),
    );
    let profile_id = profile.id.clone();
    profiles.save(&profile).await.unwrap();

    let session = domain::models::ImportSession::new(UserId::from_uuid(user_id));
    let session_id = session.id.clone();
    sessions.create(&session).await.unwrap();

    apply_profile::execute(
        Arc::clone(&profiles) as _,
        Arc::clone(&sessions) as _,
        ApplyImportProfileCommand {
            user_id,
            session_id: session_id.value(),
            profile_id: profile_id.value(),
        },
    )
    .await
    .unwrap();

    // Verify the session got updated with field_mappings and row_results cleared
    let updated = sessions
        .get(&session_id, &UserId::from_uuid(user_id))
        .await
        .unwrap()
        .unwrap();
    assert!(updated.field_mappings.is_some());
    assert_eq!(updated.field_mappings.unwrap().len(), 1);
    assert!(updated.row_results.is_none());
}
