use domain::value_objects::Username;

use crate::test_helpers::TestContextBuilder;
use crate::users::deps::ResolveUsernameDeps;
use crate::users::resolve_username_to_id;

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
