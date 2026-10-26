use std::sync::Arc;

use uuid::Uuid;

use domain::models::import::{ImportRow, ParsedFile, RowResult};
use domain::models::{AnnotatedRow, ImportSession};
use domain::ports::ImportSessionRepository;
use domain::testing::InMemoryStore;
use domain::value_objects::{ImportSessionId, UserId};

use application::import::deps::GetPreviewStageDeps;
use application::import::get_preview_stage::{self, PreviewStage};

#[tokio::test]
async fn get_preview_stage_reports_not_yet_mapped_when_row_results_absent() {
    let sessions = InMemoryStore::new();
    let user_id = Uuid::new_v4();
    let session = ImportSession::new(UserId::from_uuid(user_id));
    let session_id = session.id.clone();
    sessions.create(&session).await.unwrap();

    let deps = GetPreviewStageDeps {
        import_session: Arc::clone(&sessions) as _,
    };

    let stage = get_preview_stage::execute(&deps, session_id, user_id)
        .await
        .unwrap();

    assert!(matches!(stage, PreviewStage::NotYetMapped));
}

#[tokio::test]
async fn get_preview_stage_returns_rows_once_mapped() {
    let sessions = InMemoryStore::new();
    let user_id = Uuid::new_v4();
    let mut session = ImportSession::new(UserId::from_uuid(user_id));
    session.parsed_file = Some(ParsedFile {
        columns: vec!["Name".into()],
        rows: vec![vec!["Test".into()]],
    });
    session.row_results = Some(vec![AnnotatedRow {
        result: RowResult::Valid(ImportRow {
            title: Some("Test".into()),
            ..ImportRow::default()
        }),
        is_duplicate: false,
    }]);
    let session_id = session.id.clone();
    sessions.create(&session).await.unwrap();

    let deps = GetPreviewStageDeps {
        import_session: Arc::clone(&sessions) as _,
    };

    let stage = get_preview_stage::execute(&deps, session_id, user_id)
        .await
        .unwrap();

    match stage {
        PreviewStage::Ready(preview) => {
            assert_eq!(preview.columns, vec!["Name".to_string()]);
            assert_eq!(preview.rows.len(), 1);
        }
        PreviewStage::NotYetMapped => panic!("expected Ready, got NotYetMapped"),
    }
}

#[tokio::test]
async fn get_preview_stage_is_not_found_when_session_missing() {
    let sessions = InMemoryStore::new();
    let deps = GetPreviewStageDeps {
        import_session: Arc::clone(&sessions) as _,
    };

    let result =
        get_preview_stage::execute(&deps, ImportSessionId::generate(), Uuid::new_v4()).await;

    assert!(result.is_err());
}
