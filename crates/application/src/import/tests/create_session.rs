use std::sync::Arc;

use uuid::Uuid;

use domain::testing::InMemoryImportSessionRepository;

use crate::import::deps::CreateSessionDeps;
use crate::import::{commands::CreateImportSessionCommand, create_session};
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn creates_session_with_parsed_file() {
    let sessions = InMemoryImportSessionRepository::new();
    let b = TestContextBuilder::new();

    let deps = CreateSessionDeps {
        import_session: Arc::clone(&sessions) as _,
        document_parser: b.document_parser.clone(),
    };

    let result = create_session::execute(
        &deps,
        CreateImportSessionCommand {
            user_id: Uuid::new_v4(),
            bytes: b"col1\nval1".to_vec(),
            format: domain::models::FileFormat::Csv,
        },
    )
    .await
    .unwrap();

    assert!(!result.columns.is_empty());
}
