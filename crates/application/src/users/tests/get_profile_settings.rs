use domain::models::{ProfileField, UserRole};
use domain::value_objects::{Email, UserId};

use crate::auth::commands::RegisterCommand;
use crate::auth::deps::RegisterDeps;
use crate::auth::register;
use crate::test_helpers::TestContextBuilder;
use crate::users::deps::GetProfileSettingsDeps;
use crate::users::get_profile_settings;

/// `get_profile_settings` absorbs BOTH of the handler's calls: the user lookup
/// (`repos.user.find_by_id`) and the profile_fields join
/// (`repos.profile_fields.get_fields`). This proves the join actually moved in,
/// not just the user lookup.
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
        RegisterCommand {
            email: "settings@test.com".into(),
            username: "settingsuser".into(),
            password: "password123".into(),
            role: UserRole::Standard,
        },
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
