use uuid::Uuid;

use crate::import::{commands::CreateImportSessionCommand, create_session};
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn creates_session_with_parsed_file() {
    let ctx = TestContextBuilder::new().build();

    let result = create_session::execute(
        &ctx,
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
