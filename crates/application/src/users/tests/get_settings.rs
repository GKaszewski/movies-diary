use uuid::Uuid;

use crate::{test_helpers::TestContextBuilder, users::get_settings};

#[tokio::test]
async fn returns_default_settings() {
    let ctx = TestContextBuilder::new().build();

    let settings = get_settings::execute(&ctx, Uuid::nil()).await.unwrap();

    assert!(!settings.federate_goals());
}
