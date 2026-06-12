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
    let b = TestContextBuilder::new().with_user_settings(Arc::clone(&settings_repo) as _);
    let user_settings = b.user_settings_repo.clone();
    let uid = Uuid::nil();

    crate::users::update_settings::execute(
        user_settings.clone(),
        UpdateUserSettingsCommand {
            user_id: uid,
            federate_goals: false,
            federate_reviews: true,
            federate_watchlist: true,
        },
    )
    .await
    .unwrap();

    let settings = get_settings::execute(user_settings, uid).await.unwrap();
    assert!(!settings.federate_goals());
    assert!(settings.federate_reviews());
    assert!(settings.federate_watchlist());
}

#[tokio::test]
async fn updates_federate_reviews() {
    let settings_repo = InMemoryUserSettingsRepository::new();
    let b = TestContextBuilder::new().with_user_settings(Arc::clone(&settings_repo) as _);
    let user_settings = b.user_settings_repo.clone();
    let uid = Uuid::nil();

    crate::users::update_settings::execute(
        user_settings.clone(),
        UpdateUserSettingsCommand {
            user_id: uid,
            federate_goals: true,
            federate_reviews: false,
            federate_watchlist: true,
        },
    )
    .await
    .unwrap();

    let settings = get_settings::execute(user_settings, uid).await.unwrap();
    assert!(settings.federate_goals());
    assert!(!settings.federate_reviews());
    assert!(settings.federate_watchlist());
}

#[tokio::test]
async fn updates_federate_watchlist() {
    let settings_repo = InMemoryUserSettingsRepository::new();
    let b = TestContextBuilder::new().with_user_settings(Arc::clone(&settings_repo) as _);
    let user_settings = b.user_settings_repo.clone();
    let uid = Uuid::nil();

    crate::users::update_settings::execute(
        user_settings.clone(),
        UpdateUserSettingsCommand {
            user_id: uid,
            federate_goals: true,
            federate_reviews: true,
            federate_watchlist: false,
        },
    )
    .await
    .unwrap();

    let settings = get_settings::execute(user_settings, uid).await.unwrap();
    assert!(settings.federate_goals());
    assert!(settings.federate_reviews());
    assert!(!settings.federate_watchlist());
}
