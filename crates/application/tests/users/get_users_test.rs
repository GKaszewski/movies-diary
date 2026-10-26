use crate::support::TestContextBuilder;
use domain::testing::test_user;

use application::users::deps::GetUsersListDeps;
use application::users::get_users;
use application::users::queries::GetUsersQuery;

fn deps(b: &TestContextBuilder) -> GetUsersListDeps {
    GetUsersListDeps {
        user: b.user_repo.clone(),
        federation_admin: b.federation_admin.clone(),
    }
}

#[tokio::test]
async fn returns_empty_lists_when_nothing_exists() {
    let b = TestContextBuilder::new();

    let result = get_users::execute(&deps(&b), GetUsersQuery).await.unwrap();

    assert!(result.users.is_empty());
    assert!(result.remote_actors.is_empty());
}

#[tokio::test]
async fn lists_saved_user() {
    let b = TestContextBuilder::new();
    let user = test_user("alice");
    b.user_repo.save(&user).await.unwrap();

    let result = get_users::execute(&deps(&b), GetUsersQuery).await.unwrap();

    assert_eq!(result.users.len(), 1);
    assert_eq!(result.users[0].username(), "alice");
    assert!(result.remote_actors.is_empty());
}
