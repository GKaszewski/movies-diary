use crate::support::TestContextBuilder;
use crate::support::update_user_settings_command;
use std::sync::Arc;

use domain::testing::InMemoryStore;
use uuid::Uuid;

use application::users::{
    deps::{GetSettingsDeps, UpdateSettingsDeps},
    get_settings,
    update_settings::UpdateUserSettingsCommand,
};

#[tokio::test]
async fn updates_federate_goals() {
    let settings_repo = InMemoryStore::new();
    let b = TestContextBuilder::new().with_user_settings(Arc::clone(&settings_repo) as _);
    let user_settings = b.user_settings_repo.clone();
    let uid = Uuid::nil();

    application::users::update_settings::execute(
        &UpdateSettingsDeps {
            user_settings: user_settings.clone(),
        },
        UpdateUserSettingsCommand {
            federate_goals: false,
            ..update_user_settings_command(uid)
        },
    )
    .await
    .unwrap();

    let settings = get_settings::execute(&GetSettingsDeps { user_settings }, uid)
        .await
        .unwrap();
    assert!(!settings.federate_goals());
    assert!(settings.federate_reviews());
    assert!(settings.federate_watchlist());
}

#[tokio::test]
async fn updates_federate_reviews() {
    let settings_repo = InMemoryStore::new();
    let b = TestContextBuilder::new().with_user_settings(Arc::clone(&settings_repo) as _);
    let user_settings = b.user_settings_repo.clone();
    let uid = Uuid::nil();

    application::users::update_settings::execute(
        &UpdateSettingsDeps {
            user_settings: user_settings.clone(),
        },
        UpdateUserSettingsCommand {
            federate_reviews: false,
            ..update_user_settings_command(uid)
        },
    )
    .await
    .unwrap();

    let settings = get_settings::execute(&GetSettingsDeps { user_settings }, uid)
        .await
        .unwrap();
    assert!(settings.federate_goals());
    assert!(!settings.federate_reviews());
    assert!(settings.federate_watchlist());
}

#[tokio::test]
async fn updates_federate_watchlist() {
    let settings_repo = InMemoryStore::new();
    let b = TestContextBuilder::new().with_user_settings(Arc::clone(&settings_repo) as _);
    let user_settings = b.user_settings_repo.clone();
    let uid = Uuid::nil();

    application::users::update_settings::execute(
        &UpdateSettingsDeps {
            user_settings: user_settings.clone(),
        },
        UpdateUserSettingsCommand {
            federate_watchlist: false,
            ..update_user_settings_command(uid)
        },
    )
    .await
    .unwrap();

    let settings = get_settings::execute(&GetSettingsDeps { user_settings }, uid)
        .await
        .unwrap();
    assert!(settings.federate_goals());
    assert!(settings.federate_reviews());
    assert!(!settings.federate_watchlist());
}
