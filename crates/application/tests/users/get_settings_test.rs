use crate::support::TestContextBuilder;
use uuid::Uuid;

use application::users::{deps::GetSettingsDeps, get_settings};

#[tokio::test]
async fn returns_default_settings() {
    let b = TestContextBuilder::new();
    let deps = GetSettingsDeps {
        user_settings: b.user_settings_repo.clone(),
    };

    let settings = get_settings::execute(&deps, Uuid::nil()).await.unwrap();

    assert!(settings.federate_goals());
    assert!(settings.federate_reviews());
    assert!(settings.federate_watchlist());
}
