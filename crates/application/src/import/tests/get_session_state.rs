use std::sync::Arc;

use uuid::Uuid;

use domain::models::ImportSession;
use domain::models::import::ParsedFile;
use domain::ports::ImportSessionRepository;
use domain::testing::InMemoryImportSessionRepository;
use domain::value_objects::{ImportSessionId, UserId};

use crate::import::deps::GetSessionStateDeps;
use crate::import::get_session_state;

#[tokio::test]
async fn get_session_state_is_not_found_when_session_missing() {
    let sessions = InMemoryImportSessionRepository::new();
    let deps = GetSessionStateDeps {
        import_session: Arc::clone(&sessions) as _,
    };

    let result =
        get_session_state::execute(&deps, ImportSessionId::generate(), Uuid::new_v4()).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn get_session_state_reports_columns_row_count_and_mapping_status() {
    let sessions = InMemoryImportSessionRepository::new();
    let user_id = Uuid::new_v4();
    let mut session = ImportSession::new(UserId::from_uuid(user_id));
    session.parsed_file = Some(ParsedFile {
        columns: vec!["Name".into()],
        rows: vec![vec!["a".into()], vec!["b".into()]],
    });
    let session_id = session.id.clone();
    sessions.create(&session).await.unwrap();

    let deps = GetSessionStateDeps {
        import_session: Arc::clone(&sessions) as _,
    };

    let state = get_session_state::execute(&deps, session_id, user_id)
        .await
        .unwrap();

    assert_eq!(state.columns, vec!["Name".to_string()]);
    assert_eq!(state.row_count, 2);
    assert!(!state.has_mappings);
}
