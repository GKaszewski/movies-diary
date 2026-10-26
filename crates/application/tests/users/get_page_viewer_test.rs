use crate::support::register_command;
use domain::value_objects::{Email, FollowTarget, SocialIdentity, UserId};

use crate::support::TestContextBuilder;
use application::auth::deps::RegisterDeps;
use application::auth::register;
use application::users::deps::GetPageViewerDeps;
use application::users::get_page_viewer;

async fn setup_user(b: &TestContextBuilder, email: &str, username: &str) -> uuid::Uuid {
    let deps = RegisterDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        config: b.config.clone(),
    };
    register::execute(&deps, register_command(email, username))
        .await
        .unwrap();
    let user = b
        .user_repo
        .find_by_email(&Email::new(email.into()).unwrap())
        .await
        .unwrap()
        .unwrap();
    user.id().value()
}

#[tokio::test]
async fn get_page_viewer_reports_pending_follower_count() {
    let b = TestContextBuilder::new();
    let owner_uuid = setup_user(&b, "owner@test.com", "pageowner").await;
    let follower1 = setup_user(&b, "follower1@test.com", "pagefollower1").await;
    let follower2 = setup_user(&b, "follower2@test.com", "pagefollower2").await;

    let owner_id = UserId::from_uuid(owner_uuid);
    let target = FollowTarget::Identity(SocialIdentity::Local(owner_id.clone()));
    b.social_command
        .follow(&UserId::from_uuid(follower1), &target, &[])
        .await
        .unwrap();
    b.social_command
        .follow(&UserId::from_uuid(follower2), &target, &[])
        .await
        .unwrap();

    let deps = GetPageViewerDeps {
        user: b.user_repo.clone(),
        follow_graph: b.social_query_unified.clone(),
    };

    let v = get_page_viewer::execute(&deps, owner_uuid).await.unwrap();
    assert_eq!(v.pending_follow_count, 2);
    assert!(!v.is_admin);
}
