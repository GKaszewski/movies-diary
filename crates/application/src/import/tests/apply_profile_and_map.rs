use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use domain::models::import::{DomainField, Transform};
use domain::models::{FieldMapping, FileFormat, ImportProfile};
use domain::ports::{ImportProfileRepository, ImportSessionRepository};
use domain::testing::{InMemoryImportProfileRepository, InMemoryImportSessionRepository};
use domain::value_objects::{ImportProfileId, UserId};

use crate::import::deps::{ApplyProfileAndMapDeps, CreateSessionDeps};
use crate::import::{
    apply_profile_and_map, commands::ApplyProfileAndMapCommand,
    commands::CreateImportSessionCommand, create_session,
};
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn fails_when_profile_not_found() {
    let profiles = InMemoryImportProfileRepository::new();
    let sessions = InMemoryImportSessionRepository::new();
    let b = TestContextBuilder::new();

    let deps = ApplyProfileAndMapDeps {
        import_profile: Arc::clone(&profiles) as _,
        import_session: Arc::clone(&sessions) as _,
        document_parser: b.document_parser.clone(),
        movie_query: b.movie_query.clone(),
    };

    let result = apply_profile_and_map::execute(
        &deps,
        ApplyProfileAndMapCommand {
            user_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            profile_id: Uuid::new_v4(),
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn applies_profile_then_regenerates_mapping() {
    let profiles = InMemoryImportProfileRepository::new();
    let sessions = InMemoryImportSessionRepository::new();
    let b = TestContextBuilder::new();
    let user_id = Uuid::new_v4();

    let profile = ImportProfile::new(
        ImportProfileId::generate(),
        UserId::from_uuid(user_id),
        "letterboxd".into(),
        vec![FieldMapping {
            source_column: "title".into(),
            domain_field: DomainField::Title,
            transform: Transform::Identity,
        }],
        Utc::now().naive_utc(),
    );
    let profile_id = profile.id.clone();
    profiles.save(&profile).await.unwrap();

    let create_deps = CreateSessionDeps {
        import_session: Arc::clone(&sessions) as _,
        document_parser: b.document_parser.clone(),
    };
    let created = create_session::execute(
        &create_deps,
        CreateImportSessionCommand {
            user_id,
            bytes: b"title\nTest".to_vec(),
            format: FileFormat::Csv,
        },
    )
    .await
    .unwrap();

    let deps = ApplyProfileAndMapDeps {
        import_profile: Arc::clone(&profiles) as _,
        import_session: Arc::clone(&sessions) as _,
        document_parser: b.document_parser.clone(),
        movie_query: b.movie_query.clone(),
    };

    let rows = apply_profile_and_map::execute(
        &deps,
        ApplyProfileAndMapCommand {
            user_id,
            session_id: created.session_id.value(),
            profile_id: profile_id.value(),
        },
    )
    .await
    .unwrap();

    assert!(!rows.is_empty());

    let updated = sessions
        .get(&created.session_id, &UserId::from_uuid(user_id))
        .await
        .unwrap()
        .unwrap();
    assert!(updated.row_results.is_some());
    assert!(updated.field_mappings.is_some());
}
