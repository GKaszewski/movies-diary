use std::sync::Arc;

use domain::testing::InMemoryUserSettingsRepository;
use uuid::Uuid;

use crate::{
    test_helpers::TestContextBuilder,
    users::{get_settings, update_settings::UpdateUserSettingsCommand},
};

#[tokio::test]
async fn updates_federate_goals() {
    let settings_repo = InMemoryUserSettingsRepository::new();
    let ctx = TestContextBuilder::new()
        .with_user_settings(Arc::clone(&settings_repo) as _)
        .build();

    let uid = Uuid::nil();

    crate::users::update_settings::execute(
        &ctx,
        UpdateUserSettingsCommand {
            user_id: uid,
            federate_goals: true,
        },
    )
    .await
    .unwrap();

    let settings = get_settings::execute(&ctx, uid).await.unwrap();
    assert!(settings.federate_goals());
}
