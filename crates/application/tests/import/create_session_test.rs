use crate::support::create_import_session_command;
use std::sync::Arc;

use uuid::Uuid;

use domain::testing::InMemoryStore;

use crate::support::TestContextBuilder;
use application::import::create_session;
use application::import::deps::CreateSessionDeps;

#[tokio::test]
async fn creates_session_with_parsed_file() {
    let sessions = InMemoryStore::new();
    let b = TestContextBuilder::new();

    let deps = CreateSessionDeps {
        import_session: Arc::clone(&sessions) as _,
        document_parser: b.document_parser.clone(),
    };

    let result = create_session::execute(
        &deps,
        create_import_session_command(Uuid::new_v4(), b"col1\nval1"),
    )
    .await
    .unwrap();

    assert!(!result.columns.is_empty());
}
