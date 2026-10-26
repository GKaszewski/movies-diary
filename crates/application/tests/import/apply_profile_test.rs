use crate::support::apply_import_profile_command;
use std::sync::Arc;

use chrono::Utc;
use domain::models::ImportProfile;
use domain::ports::{ImportProfileRepository, ImportSessionRepository};
use domain::testing::InMemoryStore;
use domain::value_objects::{ImportProfileId, UserId};
use uuid::Uuid;

use application::import::apply_profile;
use application::import::deps::ApplyProfileDeps;

#[tokio::test]
async fn fails_when_profile_not_found() {
    let profiles = InMemoryStore::new();
    let sessions = Arc::clone(&profiles);

    let deps = ApplyProfileDeps {
        import_profile: Arc::clone(&profiles) as _,
        import_session: Arc::clone(&sessions) as _,
    };

    let result = apply_profile::execute(
        &deps,
        apply_import_profile_command(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()),
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn fails_when_session_not_found() {
    let profiles = InMemoryStore::new();
    let sessions = Arc::clone(&profiles);
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

    let deps = ApplyProfileDeps {
        import_profile: Arc::clone(&profiles) as _,
        import_session: Arc::clone(&sessions) as _,
    };

    let result = apply_profile::execute(
        &deps,
        apply_import_profile_command(user_id, Uuid::new_v4(), profile_id.value()),
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn applies_profile_mappings_to_session() {
    let profiles = InMemoryStore::new();
    let sessions = domain::testing::InMemoryStore::new();
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

    let deps = ApplyProfileDeps {
        import_profile: Arc::clone(&profiles) as _,
        import_session: Arc::clone(&sessions) as _,
    };

    apply_profile::execute(
        &deps,
        apply_import_profile_command(user_id, session_id.value(), profile_id.value()),
    )
    .await
    .unwrap();

    // Verify the session got updated with field_mappings and row_results cleared
    let updated =
        ImportSessionRepository::get(&*sessions, &session_id, &UserId::from_uuid(user_id))
            .await
            .unwrap()
            .unwrap();
    assert!(updated.field_mappings.is_some());
    assert_eq!(updated.field_mappings.unwrap().len(), 1);
    assert!(updated.row_results.is_none());
}
