use crate::test_helpers::TestContextBuilder;
use crate::users::get_users;
use crate::users::queries::GetUsersQuery;

#[tokio::test]
async fn returns_empty_when_no_users() {
    let ctx = TestContextBuilder::new().build();

    let result = get_users::execute(&ctx, GetUsersQuery).await.unwrap();

    assert!(result.users.is_empty());
    assert!(result.remote_actors.is_empty());
}
