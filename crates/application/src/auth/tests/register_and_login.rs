use crate::auth::commands::RegisterAndLoginCommand;
use crate::auth::register_and_login;
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn registers_and_returns_token() {
    let ctx = TestContextBuilder::new().build();

    let result = register_and_login::execute(
        &ctx,
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
