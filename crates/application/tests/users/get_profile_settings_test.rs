use crate::support::register_command;
use domain::models::ProfileField;
use domain::value_objects::{Email, UserId};

use crate::support::TestContextBuilder;
use application::auth::deps::RegisterDeps;
use application::auth::register;
use application::users::deps::GetProfileSettingsDeps;
use application::users::get_profile_settings;

#[tokio::test]
async fn get_profile_settings_includes_profile_fields() {
    let b = TestContextBuilder::new();
    let register_deps = RegisterDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        config: b.config.clone(),
    };
    register::execute(
        &register_deps,
        register_command("settings@test.com", "settingsuser"),
    )
    .await
    .unwrap();
    let user = b
        .user_repo
        .find_by_email(&Email::new("settings@test.com".into()).unwrap())
        .await
        .unwrap()
        .unwrap();
    let uid = user.id().value();

    b.profile_fields_repo
        .set_fields(
            &UserId::from_uuid(uid),
            vec![ProfileField {
                name: "pronouns".into(),
                value: "they/them".into(),
            }],
            &[],
        )
        .await
        .unwrap();

    let deps = GetProfileSettingsDeps {
        user: b.user_repo.clone(),
        profile_fields: b.profile_fields_repo.clone(),
        instance: domain::value_objects::InstanceIdentity::new(b.config.base_url.clone()),
    };

    let s = get_profile_settings::execute(&deps, uid).await.unwrap();
    assert_eq!(
        s.fields,
        vec![("pronouns".to_string(), "they/them".to_string())]
    );
}
