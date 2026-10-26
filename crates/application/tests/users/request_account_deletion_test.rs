use std::sync::Arc;

use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::UserRole,
    ports::UserRepository,
    testing::InMemoryStore,
    value_objects::{Email, UserId},
};

use application::{
    auth::{deps::RegisterDeps, register},
    users::{deps::RequestAccountDeletionDeps, request_account_deletion},
};

use crate::support::{TestContextBuilder, register_command};

use async_trait::async_trait;
use domain::models::{User, UserSummary};
use domain::ports::UserRepository as UserRepositoryPort;
use domain::value_objects::Username;

struct StaleAdminCount(Arc<InMemoryStore>);

#[async_trait]
impl UserRepositoryPort for StaleAdminCount {
    async fn find_by_email(&self, e: &Email) -> Result<Option<User>, DomainError> {
        self.0.find_by_email(e).await
    }
    async fn find_by_username(&self, u: &Username) -> Result<Option<User>, DomainError> {
        self.0.find_by_username(u).await
    }
    async fn save(&self, u: &User) -> Result<(), DomainError> {
        UserRepositoryPort::save(&*self.0, u).await
    }
    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, DomainError> {
        self.0.find_by_id(id).await
    }
    async fn list_with_stats(&self) -> Result<Vec<UserSummary>, DomainError> {
        self.0.list_with_stats().await
    }
    async fn update_profile(
        &self,
        id: &UserId,
        p: &domain::models::UserProfile,
        ev: &[DomainEvent],
    ) -> Result<(), DomainError> {
        self.0.update_profile(id, p, ev).await
    }
    async fn mark_deleted(&self, id: &UserId, ev: &[DomainEvent]) -> Result<(), DomainError> {
        self.0.mark_deleted(id, ev).await
    }
    async fn find_tombstoned_by_id(&self, id: &UserId) -> Result<Option<User>, DomainError> {
        self.0.find_tombstoned_by_id(id).await
    }
    async fn is_username_retired(&self, u: &Username) -> Result<bool, DomainError> {
        self.0.is_username_retired(u).await
    }
    async fn count_active_admins(&self) -> Result<u64, DomainError> {
        Ok(2)
    }
}

const PASSWORD: &str = "password123";

struct Fixture {
    store: Arc<InMemoryStore>,
    deps: RequestAccountDeletionDeps,
    register: RegisterDeps,
}

fn fixture() -> Fixture {
    let store = InMemoryStore::new();
    let b = TestContextBuilder::new().with_users(Arc::clone(&store) as _);
    Fixture {
        deps: RequestAccountDeletionDeps {
            user: b.user_repo.clone(),
            password_hasher: b.password_hasher.clone(),
        },
        register: RegisterDeps {
            user: b.user_repo.clone(),
            password_hasher: b.password_hasher.clone(),
            config: b.config.clone(),
        },
        store,
    }
}

impl Fixture {
    async fn user(&self, name: &str, role: UserRole) -> UserId {
        let email = format!("{name}@example.com");
        let mut cmd = register_command(email.clone(), name);
        cmd.role = role;
        register::execute(&self.register, cmd).await.unwrap();
        self.store
            .find_by_email(&Email::new(email).unwrap())
            .await
            .unwrap()
            .unwrap()
            .id()
            .clone()
    }

    async fn is_live(&self, id: &UserId) -> bool {
        self.store.find_by_id(id).await.unwrap().is_some()
    }
}

#[tokio::test]
async fn a_user_deletes_their_own_account_with_the_right_password() {
    let f = fixture();
    let alice = f.user("alice", UserRole::Standard).await;

    request_account_deletion::execute(&f.deps, alice.clone(), alice.clone(), PASSWORD)
        .await
        .unwrap();

    assert!(!f.is_live(&alice).await);
    assert!(
        matches!(
            f.store.recorded_events().as_slice(),
            [DomainEvent::UserDeleted { user_id }] if *user_id == alice
        ),
        "events: {:?}",
        f.store.recorded_events()
    );
}

#[tokio::test]
async fn a_wrong_password_is_unauthorized_and_changes_nothing() {
    let f = fixture();
    let alice = f.user("alice", UserRole::Standard).await;

    let err = request_account_deletion::execute(&f.deps, alice.clone(), alice.clone(), "nope")
        .await
        .unwrap_err();

    assert!(
        matches!(err, DomainError::InvalidCredentials(_)),
        "got {err:?}"
    );
    assert!(f.is_live(&alice).await);
    assert!(f.store.recorded_events().is_empty());
}

#[tokio::test]
async fn a_non_admin_cannot_delete_someone_else() {
    let f = fixture();
    let alice = f.user("alice", UserRole::Standard).await;
    let bob = f.user("bob", UserRole::Standard).await;

    let err = request_account_deletion::execute(&f.deps, alice, bob.clone(), PASSWORD)
        .await
        .unwrap_err();

    assert!(matches!(err, DomainError::Forbidden(_)), "got {err:?}");
    assert!(f.is_live(&bob).await);
}

#[tokio::test]
async fn an_admin_deletes_another_user_with_their_own_password() {
    let f = fixture();
    let admin = f.user("root", UserRole::Admin).await;
    let bob = f.user("bob", UserRole::Standard).await;

    let err = request_account_deletion::execute(&f.deps, admin.clone(), bob.clone(), "bobs-pw")
        .await
        .unwrap_err();
    assert!(
        matches!(err, DomainError::InvalidCredentials(_)),
        "got {err:?}"
    );
    assert!(f.is_live(&bob).await);

    request_account_deletion::execute(&f.deps, admin.clone(), bob.clone(), PASSWORD)
        .await
        .unwrap();

    assert!(!f.is_live(&bob).await);
    assert!(f.is_live(&admin).await);
}

#[tokio::test]
async fn the_last_active_admin_cannot_delete_themselves() {
    let f = fixture();
    let admin = f.user("root", UserRole::Admin).await;

    let err = request_account_deletion::execute(&f.deps, admin.clone(), admin.clone(), PASSWORD)
        .await
        .unwrap_err();

    assert!(matches!(err, DomainError::Conflict(_)), "got {err:?}");
    assert!(f.is_live(&admin).await);
    assert!(f.store.recorded_events().is_empty());
}

#[tokio::test]
async fn an_admin_can_delete_themselves_when_another_admin_remains() {
    let f = fixture();
    let first = f.user("root", UserRole::Admin).await;
    let _second = f.user("root2", UserRole::Admin).await;

    request_account_deletion::execute(&f.deps, first.clone(), first.clone(), PASSWORD)
        .await
        .unwrap();

    assert!(!f.is_live(&first).await);
}

#[tokio::test]
async fn an_unknown_target_is_not_found() {
    let f = fixture();
    let admin = f.user("root", UserRole::Admin).await;

    let err = request_account_deletion::execute(&f.deps, admin, UserId::generate(), PASSWORD)
        .await
        .unwrap_err();

    assert!(matches!(err, DomainError::NotFound(_)), "got {err:?}");
}

#[tokio::test]
async fn an_already_deleted_target_is_not_found() {
    let f = fixture();
    let admin = f.user("root", UserRole::Admin).await;
    let bob = f.user("bob", UserRole::Standard).await;
    request_account_deletion::execute(&f.deps, admin.clone(), bob.clone(), PASSWORD)
        .await
        .unwrap();

    let err = request_account_deletion::execute(&f.deps, admin, bob, PASSWORD)
        .await
        .unwrap_err();

    assert!(matches!(err, DomainError::NotFound(_)), "got {err:?}");
}

#[tokio::test]
async fn a_conflict_from_the_repository_is_propagated() {
    let f = fixture();
    let admin = f.user("root", UserRole::Admin).await;
    let deps = RequestAccountDeletionDeps {
        user: Arc::new(StaleAdminCount(Arc::clone(&f.store))),
        password_hasher: f.deps.password_hasher.clone(),
    };

    let err = request_account_deletion::execute(&deps, admin.clone(), admin.clone(), PASSWORD)
        .await
        .unwrap_err();

    assert!(matches!(err, DomainError::Conflict(_)), "got {err:?}");
    assert!(f.is_live(&admin).await);
}

#[tokio::test]
async fn an_unknown_or_deleted_actor_is_unauthorized() {
    let f = fixture();
    let admin = f.user("root", UserRole::Admin).await;
    let bob = f.user("bob", UserRole::Standard).await;

    let err = request_account_deletion::execute(&f.deps, UserId::generate(), bob.clone(), PASSWORD)
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::Unauthorized(_)), "got {err:?}");

    request_account_deletion::execute(&f.deps, admin.clone(), bob.clone(), PASSWORD)
        .await
        .unwrap();
    let err = request_account_deletion::execute(&f.deps, bob.clone(), bob, PASSWORD)
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::Unauthorized(_)), "got {err:?}");
}

#[tokio::test]
async fn a_non_admin_gets_forbidden_before_any_password_check() {
    let f = fixture();
    let alice = f.user("alice", UserRole::Standard).await;
    let bob = f.user("bob", UserRole::Standard).await;

    let err = request_account_deletion::execute(&f.deps, alice, bob, "wrong")
        .await
        .unwrap_err();

    assert!(matches!(err, DomainError::Forbidden(_)), "got {err:?}");
}

#[tokio::test]
async fn the_service_actor_cannot_be_deleted_even_by_an_admin() {
    let f = fixture();
    let admin = f.user("root", UserRole::Admin).await;
    let other = f.user("second", UserRole::Admin).await;
    let _ = other;

    let err = request_account_deletion::execute(&f.deps, admin, UserId::service_actor(), PASSWORD)
        .await
        .unwrap_err();

    assert!(matches!(err, DomainError::Forbidden(_)), "got {err:?}");
    assert!(f.store.recorded_events().is_empty());
}
