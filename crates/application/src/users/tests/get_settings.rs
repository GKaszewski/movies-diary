use uuid::Uuid;

use crate::{test_helpers::TestContextBuilder, users::get_settings};

#[tokio::test]
async fn returns_default_settings() {
    let b = TestContextBuilder::new();
    let user_settings = b.user_settings_repo.clone();

    let settings = get_settings::execute(user_settings, Uuid::nil()).await.unwrap();

    assert!(!settings.federate_goals());
}
