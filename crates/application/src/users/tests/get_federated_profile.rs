use std::sync::Arc;

use async_trait::async_trait;
use domain::{errors::DomainError, models::FederatedProfile, ports::FederatedProfileQuery};

use crate::users::deps::GetFederatedProfileDeps;
use crate::users::get_federated_profile;

fn a_profile() -> FederatedProfile {
    FederatedProfile {
        actor_url: "https://remote.example/users/alice".into(),
        handle: "alice@remote.example".into(),
        display_name: Some("Alice".into()),
        bio: None,
        avatar_url: None,
        banner_url: None,
    }
}

struct FoundFederatedProfileQuery;

#[async_trait]
impl FederatedProfileQuery for FoundFederatedProfileQuery {
    async fn get_federated_profile(
        &self,
        _synthetic_user_id: uuid::Uuid,
    ) -> Result<Option<FederatedProfile>, DomainError> {
        Ok(Some(a_profile()))
    }
}

struct EmptyFederatedProfileQuery;

#[async_trait]
impl FederatedProfileQuery for EmptyFederatedProfileQuery {
    async fn get_federated_profile(
        &self,
        _synthetic_user_id: uuid::Uuid,
    ) -> Result<Option<FederatedProfile>, DomainError> {
        Ok(None)
    }
}

struct FailingFederatedProfileQuery;

#[async_trait]
impl FederatedProfileQuery for FailingFederatedProfileQuery {
    async fn get_federated_profile(
        &self,
        _synthetic_user_id: uuid::Uuid,
    ) -> Result<Option<FederatedProfile>, DomainError> {
        Err(DomainError::InfrastructureError(
            "remote lookup failed".into(),
        ))
    }
}

#[tokio::test]
async fn returns_the_profile_when_the_port_finds_one() {
    let deps = GetFederatedProfileDeps {
        federated_profile: Some(Arc::new(FoundFederatedProfileQuery)),
    };

    let result = get_federated_profile::execute(&deps, uuid::Uuid::new_v4())
        .await
        .unwrap();

    assert_eq!(
        result.map(|p| p.handle),
        Some("alice@remote.example".to_string())
    );
}

#[tokio::test]
async fn returns_ok_none_when_the_port_finds_nothing() {
    let deps = GetFederatedProfileDeps {
        federated_profile: Some(Arc::new(EmptyFederatedProfileQuery)),
    };

    let result = get_federated_profile::execute(&deps, uuid::Uuid::new_v4())
        .await
        .unwrap();

    assert!(result.is_none());
}

#[tokio::test]
async fn returns_ok_none_when_federation_is_disabled() {
    let deps = GetFederatedProfileDeps {
        federated_profile: None,
    };

    let result = get_federated_profile::execute(&deps, uuid::Uuid::new_v4())
        .await
        .unwrap();

    assert!(
        result.is_none(),
        "an absent port must collapse to Ok(None), the same as an empty lookup — \
         the handler cannot and must not tell the two apart"
    );
}

#[tokio::test]
async fn propagates_an_error_from_the_port_unchanged() {
    let deps = GetFederatedProfileDeps {
        federated_profile: Some(Arc::new(FailingFederatedProfileQuery)),
    };

    let err = get_federated_profile::execute(&deps, uuid::Uuid::new_v4())
        .await
        .expect_err("a repository error must not be swallowed inside the use case");

    assert!(matches!(err, DomainError::InfrastructureError(_)));
}
