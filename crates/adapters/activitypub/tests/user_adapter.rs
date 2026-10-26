use domain::{
    ports::UserRepository,
    testing::{InMemoryStore, test_user},
    value_objects::InstanceIdentity,
};
use k_ap::ApUserRepository;

use activitypub::DomainUserRepoAdapter;

async fn adapter_with_deleted_alice() -> (DomainUserRepoAdapter, uuid::Uuid) {
    let store = InMemoryStore::new();
    let alice = test_user("alice");
    store.save(&alice).await.unwrap();
    store.mark_deleted(alice.id(), &[]).await.unwrap();
    let adapter = DomainUserRepoAdapter::new(store, InstanceIdentity::new("https://md.example"));
    (adapter, alice.id().value())
}

#[tokio::test]
async fn tombstoned_actor_can_still_sign() {
    let (adapter, id) = adapter_with_deleted_alice().await;

    let user = adapter.find_by_id(id).await.unwrap();

    assert_eq!(user.expect("k-ap resolves the tombstone").username, "alice");
}

#[tokio::test]
async fn a_tombstone_is_not_found_by_username() {
    let (adapter, _) = adapter_with_deleted_alice().await;

    assert!(adapter.find_by_username("alice").await.unwrap().is_none());
}

#[tokio::test]
async fn an_unknown_user_is_none() {
    let (adapter, _) = adapter_with_deleted_alice().await;

    assert!(
        adapter
            .find_by_id(uuid::Uuid::new_v4())
            .await
            .unwrap()
            .is_none()
    );
}

#[test]
fn every_per_user_path_is_gated() {
    use activitypub::tombstone_gate::gated_user;
    let id = uuid::Uuid::new_v4();
    let rid = uuid::Uuid::new_v4();
    for endpoint in [
        "outbox",
        "featured",
        "inbox",
        "followers",
        "following",
        &format!("reviews/{rid}"),
    ] {
        assert_eq!(gated_user(&format!("/users/{id}/{endpoint}")), Some(id));
    }
    assert_eq!(gated_user(&format!("/users/{id}")), None);
    assert_eq!(gated_user(&format!("/users/{id}/")), None);
    assert_eq!(gated_user("/inbox"), None);
    assert_eq!(gated_user("/users/not-a-uuid/outbox"), None);
}
