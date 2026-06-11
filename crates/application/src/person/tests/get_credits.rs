use domain::models::PersonId;
use domain::testing::{FakePersonQuery, NoopEventPublisher};
use std::sync::Arc;
use uuid::Uuid;

use crate::person::{deps::GetPersonDeps, get_credits};

#[tokio::test]
async fn returns_empty_credits() {
    let deps = GetPersonDeps {
        person_query: Arc::new(FakePersonQuery),
        event_publisher: NoopEventPublisher::new(),
    };

    let result = get_credits::execute(&deps, PersonId::from_uuid(Uuid::new_v4()))
        .await
        .unwrap();

    assert!(result.cast.is_empty());
    assert!(result.crew.is_empty());
}
