use domain::value_objects::Username;

use crate::test_helpers::TestContextBuilder;
use crate::users::deps::ResolveUsernameDeps;
use crate::users::resolve_username_to_id;

/// `resolve_username_to_id` absorbs `handlers/users.rs::get_user_by_username`'s
/// `repos.user.find_by_username()` call. Unknown usernames must resolve to `None`,
/// not an error — the handler turns `None` into a 404, same as today.
#[tokio::test]
async fn resolve_username_to_id_returns_none_for_unknown() {
    let b = TestContextBuilder::new();
    let deps = ResolveUsernameDeps {
        user: b.user_repo.clone(),
    };
    let unknown = Username::new("nosuchuser".into()).unwrap();
    assert!(
        resolve_username_to_id::execute(&deps, &unknown)
            .await
            .unwrap()
            .is_none()
    );
}
