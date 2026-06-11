use std::sync::Arc;

use uuid::Uuid;

use domain::testing::InMemoryImportSessionRepository;

use crate::import::{commands::CreateImportSessionCommand, create_session};
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn creates_session_with_parsed_file() {
    let sessions = InMemoryImportSessionRepository::new();
    let ctx = TestContextBuilder::new().build();

    let result = create_session::execute(
        Arc::clone(&sessions) as _,
        ctx.services.document_parser.clone(),
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
