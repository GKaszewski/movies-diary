use std::sync::Arc;

use domain::events::DomainEvent;
use domain::models::ProfileField;
use domain::testing::{InMemoryProfileFieldsRepo, NoopEventPublisher};
use uuid::Uuid;

use crate::{
    test_helpers::TestContextBuilder,
    users::{commands::UpdateProfileFieldsCommand, update_profile_fields},
};

#[tokio::test]
async fn saves_profile_fields() {
    let fields_repo = InMemoryProfileFieldsRepo::new();
    let events = NoopEventPublisher::new();
    let b = TestContextBuilder::new()
        .with_profile_fields(Arc::clone(&fields_repo) as _)
        .with_event_publisher(Arc::clone(&events) as _);
    let profile_fields = b.profile_fields_repo.clone();
    let event_publisher = b.event_publisher.clone();

    update_profile_fields::execute(
        profile_fields,
        event_publisher,
        UpdateProfileFieldsCommand {
            user_id: Uuid::nil(),
            fields: vec![
                ProfileField {
                    name: "Website".into(),
                    value: "https://example.com".into(),
                },
                ProfileField {
                    name: "Location".into(),
                    value: "Berlin".into(),
                },
            ],
        },
    )
    .await
    .unwrap();

    let published = events.published();
    assert!(
        published
            .iter()
            .any(|e| matches!(e, DomainEvent::UserUpdated { .. }))
    );
}

#[tokio::test]
async fn rejects_more_than_four_fields() {
    let b = TestContextBuilder::new();
    let profile_fields = b.profile_fields_repo.clone();
    let event_publisher = b.event_publisher.clone();

    let fields: Vec<ProfileField> = (0..5)
        .map(|i| ProfileField {
            name: format!("field{i}"),
            value: format!("val{i}"),
        })
        .collect();

    let result = update_profile_fields::execute(
        profile_fields,
        event_publisher,
        UpdateProfileFieldsCommand {
            user_id: Uuid::nil(),
            fields,
        },
    )
    .await;

    assert!(result.is_err());
}
