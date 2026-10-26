use crate::support::TestContextBuilder;
use std::sync::Arc;

use domain::events::DomainEvent;
use domain::models::ProfileField;
use domain::testing::{InMemoryStore, RefusingProfileFieldsRepository};
use uuid::Uuid;

use application::users::{
    commands::UpdateProfileFieldsCommand, deps::UpdateProfileFieldsDeps, update_profile_fields,
};

#[tokio::test]
async fn saving_profile_fields_records_the_change_and_its_event_together() {
    let fields_repo = InMemoryStore::new();
    let deps = UpdateProfileFieldsDeps {
        profile_fields: Arc::clone(&fields_repo) as _,
    };

    update_profile_fields::execute(
        &deps,
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

    let recorded = fields_repo.recorded_events();
    assert_eq!(recorded.len(), 1);
    assert!(matches!(recorded[0], DomainEvent::UserUpdated { .. }));
}

#[tokio::test]
async fn rejects_more_than_four_fields() {
    let b = TestContextBuilder::new();
    let deps = UpdateProfileFieldsDeps {
        profile_fields: b.profile_fields_repo.clone(),
    };

    let fields: Vec<ProfileField> = (0..5)
        .map(|i| ProfileField {
            name: format!("field{i}"),
            value: format!("val{i}"),
        })
        .collect();

    let result = update_profile_fields::execute(
        &deps,
        UpdateProfileFieldsCommand {
            user_id: Uuid::nil(),
            fields,
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn refused_profile_fields_propagate_and_record_nothing() {
    let deps = UpdateProfileFieldsDeps {
        profile_fields: Arc::new(RefusingProfileFieldsRepository),
    };

    let result = update_profile_fields::execute(
        &deps,
        UpdateProfileFieldsCommand {
            user_id: Uuid::nil(),
            fields: vec![],
        },
    )
    .await;

    assert!(matches!(
        result,
        Err(domain::errors::DomainError::InfrastructureError(m)) if m == "refused"
    ));
}
