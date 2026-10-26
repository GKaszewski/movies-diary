use crate::support::TestContextBuilder;
use application::auth::commands::RegisterAndLoginCommand;
use application::auth::deps::RegisterAndLoginDeps;
use application::auth::register_and_login;

#[tokio::test]
async fn registers_and_returns_token() {
    let b = TestContextBuilder::new();
    let deps = RegisterAndLoginDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        auth: b.auth_service.clone(),
        refresh_session: b.refresh_session_repo.clone(),
        config: b.config.clone(),
    };

    let result = register_and_login::execute(
        &deps,
        RegisterAndLoginCommand {
            email: "new@example.com".into(),
            username: "newuser".into(),
            password: "password123".into(),
        },
    )
    .await
    .unwrap();

    assert!(!result.token.is_empty());
    assert_eq!(result.email, "new@example.com");
}
