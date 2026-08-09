use std::sync::Arc;

use uuid::Uuid;

use domain::models::ImportSession;
use domain::models::import::ParsedFile;
use domain::ports::ImportSessionRepository;
use domain::testing::InMemoryImportSessionRepository;
use domain::value_objects::{ImportSessionId, UserId};

use crate::import::deps::GetMappingStageDeps;
use crate::import::get_mapping_stage::{self, SAMPLE_ROW_LIMIT};

#[tokio::test]
async fn get_mapping_stage_is_not_found_when_file_not_parsed() {
    let sessions = InMemoryImportSessionRepository::new();
    let user_id = Uuid::new_v4();
    let session = ImportSession::new(UserId::from_uuid(user_id));
    let session_id = session.id.clone();
    sessions.create(&session).await.unwrap();

    let deps = GetMappingStageDeps {
        import_session: Arc::clone(&sessions) as _,
    };

    let result = get_mapping_stage::execute(&deps, session_id, user_id).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn get_mapping_stage_is_not_found_when_session_missing() {
    let sessions = InMemoryImportSessionRepository::new();
    let deps = GetMappingStageDeps {
        import_session: Arc::clone(&sessions) as _,
    };

    let result =
        get_mapping_stage::execute(&deps, ImportSessionId::generate(), Uuid::new_v4()).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn get_mapping_stage_returns_columns_and_capped_sample_rows() {
    let sessions = InMemoryImportSessionRepository::new();
    let user_id = Uuid::new_v4();
    let mut session = ImportSession::new(UserId::from_uuid(user_id));
    session.parsed_file = Some(ParsedFile {
        columns: vec!["Name".into(), "Year".into()],
        rows: (0..7)
            .map(|i| vec![format!("row{i}"), "2020".into()])
            .collect(),
    });
    let session_id = session.id.clone();
    sessions.create(&session).await.unwrap();

    let deps = GetMappingStageDeps {
        import_session: Arc::clone(&sessions) as _,
    };

    let stage = get_mapping_stage::execute(&deps, session_id, user_id)
        .await
        .unwrap();

    assert_eq!(stage.columns, vec!["Name".to_string(), "Year".to_string()]);
    assert_eq!(stage.sample_rows.len(), SAMPLE_ROW_LIMIT);
}
