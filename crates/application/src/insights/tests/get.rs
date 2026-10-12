use std::sync::Arc;

use chrono::NaiveDate;
use uuid::Uuid;

use domain::{models::insights::WatchRow, testing::InMemoryInsightsQuery, value_objects::UserId};

use super::*;

fn watch(genre: &str, rating: u8, day: u32, hour: u32) -> WatchRow {
    WatchRow {
        movie_id: Uuid::new_v4(),
        rating,
        watched_at: NaiveDate::from_ymd_opt(2024, 1, day)
            .unwrap()
            .and_hms_opt(hour, 0, 0)
            .unwrap(),
        watch_medium: Some("streaming".to_string()),
        genres: vec![genre.to_string()],
    }
}

#[tokio::test]
async fn returns_analyzer_output_for_the_user() {
    let user = Uuid::new_v4();
    let rows: Vec<WatchRow> = (0..12)
        .map(|i| watch("Thriller", 5, 1 + i, 22))
        .chain((0..12).map(|i| watch("Comedy", 2, 13 + i, 22)))
        .collect();
    let deps = GetInsightsDeps {
        insights: InMemoryInsightsQuery::with_rows(user, rows),
    };

    let insights = execute(&deps, UserId::from_uuid(user), &chrono::Utc)
        .await
        .unwrap();
    assert!(insights.iter().any(
        |i| matches!(i, domain::models::Insight::RatingLift { genre, .. } if genre == "Thriller")
    ));
}

#[tokio::test]
async fn other_users_rows_are_not_seen() {
    let user = Uuid::new_v4();
    let other = Uuid::new_v4();
    let rows: Vec<WatchRow> = (0..12)
        .map(|i| watch("Thriller", 5, 1 + i, 22))
        .chain((0..12).map(|i| watch("Comedy", 2, 13 + i, 22)))
        .collect();
    let deps = GetInsightsDeps {
        insights: InMemoryInsightsQuery::with_rows(other, rows),
    };

    let insights = execute(&deps, UserId::from_uuid(user), &chrono::Utc)
        .await
        .unwrap();
    assert!(insights.is_empty());
}

#[tokio::test]
async fn empty_diary_returns_empty_list() {
    let user = Uuid::new_v4();
    let deps = GetInsightsDeps {
        insights: InMemoryInsightsQuery::new(),
    };

    let insights = execute(&deps, UserId::from_uuid(user), &chrono::Utc)
        .await
        .unwrap();
    assert!(insights.is_empty());
}

#[tokio::test]
async fn port_errors_propagate() {
    struct Failing;
    #[async_trait::async_trait]
    impl domain::ports::InsightsQuery for Failing {
        async fn get_watch_rows(
            &self,
            _: &UserId,
        ) -> Result<Vec<WatchRow>, domain::errors::DomainError> {
            Err(domain::errors::DomainError::InfrastructureError(
                "db down".into(),
            ))
        }
    }

    let deps = GetInsightsDeps {
        insights: Arc::new(Failing),
    };

    let err = execute(&deps, UserId::from_uuid(Uuid::new_v4()), &chrono::Utc)
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        domain::errors::DomainError::InfrastructureError(_)
    ));
}
