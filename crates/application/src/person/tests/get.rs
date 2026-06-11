use domain::models::PersonId;
use domain::testing::{FakePersonQuery, NoopEventPublisher};
use std::sync::Arc;
use uuid::Uuid;

use crate::person::{deps::GetPersonDeps, get};

#[tokio::test]
async fn returns_none_for_unknown_person() {
    let deps = GetPersonDeps {
        person_query: Arc::new(FakePersonQuery),
        event_publisher: NoopEventPublisher::new(),
    };

    let result = get::execute(&deps, PersonId::from_uuid(Uuid::new_v4()))
        .await
        .unwrap();

    assert!(result.is_none());
}
