use crate::test_helpers::TestContextBuilder;
use crate::users::get_users;
use crate::users::queries::GetUsersQuery;

#[tokio::test]
async fn returns_empty_when_no_users() {
    let b = TestContextBuilder::new();
    let user = b.user_repo.clone();
    let social_query = b.social_query.clone();

    let result = get_users::execute(user, social_query, GetUsersQuery).await.unwrap();

    assert!(result.users.is_empty());
    assert!(result.remote_actors.is_empty());
}
