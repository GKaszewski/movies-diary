use std::sync::Arc;

use domain::models::{AnnotatedRow, ImportSession, import::RowResult};
use domain::ports::ImportSessionRepository;
use domain::testing::InMemoryImportSessionRepository;
use domain::value_objects::UserId;
use uuid::Uuid;

use crate::import::commands::ExecuteImportCommand;
use crate::import::execute;
use crate::test_helpers::NoopReviewLogger;

fn make_session_with_rows(user_id: UserId) -> ImportSession {
    let mut session = ImportSession::new(user_id);
    session.row_results = Some(vec![
        AnnotatedRow {
            result: RowResult::Valid(domain::models::ImportRow {
                title: Some("Test Movie".into()),
                release_year: Some("2024".into()),
                rating: Some("4".into()),
                watched_at: Some("2024-06-01".into()),
                external_metadata_id: None,
                director: None,
                comment: None,
            }),
            is_duplicate: false,
        },
        AnnotatedRow {
            result: RowResult::Valid(domain::models::ImportRow {
                title: Some("Another".into()),
                release_year: Some("2023".into()),
                rating: Some("3".into()),
                watched_at: Some("2024-07-01".into()),
                external_metadata_id: None,
                director: None,
                comment: None,
            }),
            is_duplicate: false,
        },
    ]);
    session
}

#[tokio::test]
async fn imports_confirmed_rows() {
    let sessions = InMemoryImportSessionRepository::new();
    let uid = Uuid::new_v4();

    let session = make_session_with_rows(UserId::from_uuid(uid));
    let sid = session.id.clone();
    sessions.create(&session).await.unwrap();

    let result = execute::execute(
        Arc::clone(&sessions) as _,
        Arc::new(NoopReviewLogger),
        ExecuteImportCommand {
            user_id: uid,
            session_id: sid.value(),
            confirmed_indices: vec![0, 1],
        },
    )
    .await
    .unwrap();

    assert_eq!(result.imported, 2);
    assert_eq!(result.skipped_duplicates, 0);
    assert!(result.failed.is_empty());
}

#[tokio::test]
async fn skips_unconfirmed_rows() {
    let sessions = InMemoryImportSessionRepository::new();
    let uid = Uuid::new_v4();

    let session = make_session_with_rows(UserId::from_uuid(uid));
    let sid = session.id.clone();
    sessions.create(&session).await.unwrap();

    let result = execute::execute(
        Arc::clone(&sessions) as _,
        Arc::new(NoopReviewLogger),
        ExecuteImportCommand {
            user_id: uid,
            session_id: sid.value(),
            confirmed_indices: vec![0],
        },
    )
    .await
    .unwrap();

    assert_eq!(result.imported, 1);
    assert_eq!(result.skipped_duplicates, 1);
}

#[tokio::test]
async fn fails_when_session_not_found() {
    let sessions = InMemoryImportSessionRepository::new();

    let result = execute::execute(
        Arc::clone(&sessions) as _,
        Arc::new(NoopReviewLogger),
        ExecuteImportCommand {
            user_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            confirmed_indices: vec![],
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn handles_datetime_format() {
    let sessions = InMemoryImportSessionRepository::new();
    let uid = Uuid::new_v4();

    let mut session = ImportSession::new(UserId::from_uuid(uid));
    let sid = session.id.clone();
    session.row_results = Some(vec![AnnotatedRow {
        result: RowResult::Valid(domain::models::ImportRow {
            title: Some("DateTime Movie".into()),
            release_year: Some("2024".into()),
            rating: Some("5".into()),
            watched_at: Some("2024-06-01T12:30:00".into()),
            external_metadata_id: None,
            director: None,
            comment: None,
        }),
        is_duplicate: false,
    }]);
    sessions.create(&session).await.unwrap();

    let result = execute::execute(
        Arc::clone(&sessions) as _,
        Arc::new(NoopReviewLogger),
        ExecuteImportCommand {
            user_id: uid,
            session_id: sid.value(),
            confirmed_indices: vec![0],
        },
    )
    .await
    .unwrap();

    assert_eq!(result.imported, 1);
    assert!(result.failed.is_empty());
}

#[tokio::test]
async fn fails_on_invalid_rating() {
    let sessions = InMemoryImportSessionRepository::new();
    let uid = Uuid::new_v4();

    let mut session = ImportSession::new(UserId::from_uuid(uid));
    let sid = session.id.clone();
    session.row_results = Some(vec![AnnotatedRow {
        result: RowResult::Valid(domain::models::ImportRow {
            title: Some("Bad Rating Movie".into()),
            release_year: Some("2024".into()),
            rating: Some("not_a_number".into()),
            watched_at: Some("2024-06-01".into()),
            external_metadata_id: None,
            director: None,
            comment: None,
        }),
        is_duplicate: false,
    }]);
    sessions.create(&session).await.unwrap();

    let result = execute::execute(
        Arc::clone(&sessions) as _,
        Arc::new(NoopReviewLogger),
        ExecuteImportCommand {
            user_id: uid,
            session_id: sid.value(),
            confirmed_indices: vec![0],
        },
    )
    .await
    .unwrap();

    assert_eq!(result.imported, 0);
    assert_eq!(result.failed.len(), 1);
}

#[tokio::test]
async fn fails_on_missing_watched_at() {
    let sessions = InMemoryImportSessionRepository::new();
    let uid = Uuid::new_v4();

    let mut session = ImportSession::new(UserId::from_uuid(uid));
    let sid = session.id.clone();
    session.row_results = Some(vec![AnnotatedRow {
        result: RowResult::Valid(domain::models::ImportRow {
            title: Some("No Date Movie".into()),
            release_year: Some("2024".into()),
            rating: Some("4".into()),
            watched_at: None,
            external_metadata_id: None,
            director: None,
            comment: None,
        }),
        is_duplicate: false,
    }]);
    sessions.create(&session).await.unwrap();

    let result = execute::execute(
        Arc::clone(&sessions) as _,
        Arc::new(NoopReviewLogger),
        ExecuteImportCommand {
            user_id: uid,
            session_id: sid.value(),
            confirmed_indices: vec![0],
        },
    )
    .await
    .unwrap();

    assert_eq!(result.imported, 0);
    assert_eq!(result.failed.len(), 1);
}

#[tokio::test]
async fn imports_row_with_external_metadata_id() {
    let sessions = InMemoryImportSessionRepository::new();
    let uid = Uuid::new_v4();

    let mut session = ImportSession::new(UserId::from_uuid(uid));
    let sid = session.id.clone();
    session.row_results = Some(vec![AnnotatedRow {
        result: RowResult::Valid(domain::models::ImportRow {
            title: Some("TMDB Movie".into()),
            release_year: Some("2024".into()),
            rating: Some("5".into()),
            watched_at: Some("2024-06-01".into()),
            external_metadata_id: Some("tt9999999".into()),
            director: None,
            comment: None,
        }),
        is_duplicate: false,
    }]);
    sessions.create(&session).await.unwrap();

    let result = execute::execute(
        Arc::clone(&sessions) as _,
        Arc::new(NoopReviewLogger),
        ExecuteImportCommand {
            user_id: uid,
            session_id: sid.value(),
            confirmed_indices: vec![0],
        },
    )
    .await
    .unwrap();

    assert_eq!(result.imported, 1);
    assert!(result.failed.is_empty());
}

#[tokio::test]
async fn imports_row_with_director_and_comment() {
    let sessions = InMemoryImportSessionRepository::new();
    let uid = Uuid::new_v4();

    let mut session = ImportSession::new(UserId::from_uuid(uid));
    let sid = session.id.clone();
    session.row_results = Some(vec![AnnotatedRow {
        result: RowResult::Valid(domain::models::ImportRow {
            title: Some("Directed Movie".into()),
            release_year: Some("2022".into()),
            rating: Some("4".into()),
            watched_at: Some("2024-06-01".into()),
            external_metadata_id: None,
            director: Some("John Director".into()),
            comment: Some("A great film".into()),
        }),
        is_duplicate: false,
    }]);
    sessions.create(&session).await.unwrap();

    let result = execute::execute(
        Arc::clone(&sessions) as _,
        Arc::new(NoopReviewLogger),
        ExecuteImportCommand {
            user_id: uid,
            session_id: sid.value(),
            confirmed_indices: vec![0],
        },
    )
    .await
    .unwrap();

    assert_eq!(result.imported, 1);
    assert!(result.failed.is_empty());
}

#[tokio::test]
async fn handles_space_separated_datetime_format() {
    let sessions = InMemoryImportSessionRepository::new();
    let uid = Uuid::new_v4();

    let mut session = ImportSession::new(UserId::from_uuid(uid));
    let sid = session.id.clone();
    session.row_results = Some(vec![AnnotatedRow {
        result: RowResult::Valid(domain::models::ImportRow {
            title: Some("Space DateTime".into()),
            release_year: Some("2024".into()),
            rating: Some("3".into()),
            watched_at: Some("2024-06-01 14:30:00".into()),
            external_metadata_id: None,
            director: None,
            comment: None,
        }),
        is_duplicate: false,
    }]);
    sessions.create(&session).await.unwrap();

    let result = execute::execute(
        Arc::clone(&sessions) as _,
        Arc::new(NoopReviewLogger),
        ExecuteImportCommand {
            user_id: uid,
            session_id: sid.value(),
            confirmed_indices: vec![0],
        },
    )
    .await
    .unwrap();

    assert_eq!(result.imported, 1);
    assert!(result.failed.is_empty());
}

#[tokio::test]
async fn reports_invalid_row_result_errors() {
    let sessions = InMemoryImportSessionRepository::new();
    let uid = Uuid::new_v4();

    let mut session = ImportSession::new(UserId::from_uuid(uid));
    let sid = session.id.clone();
    session.row_results = Some(vec![AnnotatedRow {
        result: RowResult::Invalid {
            errors: vec!["missing title".into(), "bad year".into()],
            raw: vec![("col1".into(), "val1".into())],
        },
        is_duplicate: false,
    }]);
    sessions.create(&session).await.unwrap();

    let result = execute::execute(
        Arc::clone(&sessions) as _,
        Arc::new(NoopReviewLogger),
        ExecuteImportCommand {
            user_id: uid,
            session_id: sid.value(),
            confirmed_indices: vec![0],
        },
    )
    .await
    .unwrap();

    assert_eq!(result.imported, 0);
    assert_eq!(result.failed.len(), 1);
    assert!(result.failed[0].1.contains("missing title"));
    assert!(result.failed[0].1.contains("bad year"));
}

#[tokio::test]
async fn fails_on_missing_rating() {
    let sessions = InMemoryImportSessionRepository::new();
    let uid = Uuid::new_v4();

    let mut session = ImportSession::new(UserId::from_uuid(uid));
    let sid = session.id.clone();
    session.row_results = Some(vec![AnnotatedRow {
        result: RowResult::Valid(domain::models::ImportRow {
            title: Some("No Rating Movie".into()),
            release_year: Some("2024".into()),
            rating: None,
            watched_at: Some("2024-06-01".into()),
            external_metadata_id: None,
            director: None,
            comment: None,
        }),
        is_duplicate: false,
    }]);
    sessions.create(&session).await.unwrap();

    let result = execute::execute(
        Arc::clone(&sessions) as _,
        Arc::new(NoopReviewLogger),
        ExecuteImportCommand {
            user_id: uid,
            session_id: sid.value(),
            confirmed_indices: vec![0],
        },
    )
    .await
    .unwrap();

    assert_eq!(result.imported, 0);
    assert_eq!(result.failed.len(), 1);
    assert!(result.failed[0].1.contains("missing rating"));
}

#[tokio::test]
async fn fails_on_unparseable_date() {
    let sessions = InMemoryImportSessionRepository::new();
    let uid = Uuid::new_v4();

    let mut session = ImportSession::new(UserId::from_uuid(uid));
    let sid = session.id.clone();
    session.row_results = Some(vec![AnnotatedRow {
        result: RowResult::Valid(domain::models::ImportRow {
            title: Some("Bad Date Movie".into()),
            release_year: Some("2024".into()),
            rating: Some("3".into()),
            watched_at: Some("not-a-date".into()),
            external_metadata_id: None,
            director: None,
            comment: None,
        }),
        is_duplicate: false,
    }]);
    sessions.create(&session).await.unwrap();

    let result = execute::execute(
        Arc::clone(&sessions) as _,
        Arc::new(NoopReviewLogger),
        ExecuteImportCommand {
            user_id: uid,
            session_id: sid.value(),
            confirmed_indices: vec![0],
        },
    )
    .await
    .unwrap();

    assert_eq!(result.imported, 0);
    assert_eq!(result.failed.len(), 1);
    assert!(result.failed[0].1.contains("cannot parse watched_at"));
}

#[tokio::test]
async fn imports_row_without_release_year() {
    let sessions = InMemoryImportSessionRepository::new();
    let uid = Uuid::new_v4();

    let mut session = ImportSession::new(UserId::from_uuid(uid));
    let sid = session.id.clone();
    session.row_results = Some(vec![AnnotatedRow {
        result: RowResult::Valid(domain::models::ImportRow {
            title: Some("No Year Movie".into()),
            release_year: None,
            rating: Some("4".into()),
            watched_at: Some("2024-06-01".into()),
            external_metadata_id: None,
            director: None,
            comment: None,
        }),
        is_duplicate: false,
    }]);
    sessions.create(&session).await.unwrap();

    let result = execute::execute(
        Arc::clone(&sessions) as _,
        Arc::new(NoopReviewLogger),
        ExecuteImportCommand {
            user_id: uid,
            session_id: sid.value(),
            confirmed_indices: vec![0],
        },
    )
    .await
    .unwrap();

    assert_eq!(result.imported, 1);
    assert!(result.failed.is_empty());
}

#[tokio::test]
async fn deletes_session_after_import() {
    let sessions = InMemoryImportSessionRepository::new();
    let uid = Uuid::new_v4();

    let session = make_session_with_rows(UserId::from_uuid(uid));
    let sid = session.id.clone();
    sessions.create(&session).await.unwrap();
    assert_eq!(sessions.count(), 1);

    execute::execute(
        Arc::clone(&sessions) as _,
        Arc::new(NoopReviewLogger),
        ExecuteImportCommand {
            user_id: uid,
            session_id: sid.value(),
            confirmed_indices: vec![0],
        },
    )
    .await
    .unwrap();

    assert_eq!(
        sessions.count(),
        0,
        "session should be deleted after import"
    );
}

#[tokio::test]
async fn imports_more_rows_than_concurrency_limit() {
    let sessions = InMemoryImportSessionRepository::new();
    let uid = Uuid::new_v4();

    let rows: Vec<_> = (0..15)
        .map(|i| AnnotatedRow {
            result: RowResult::Valid(domain::models::ImportRow {
                title: Some(format!("Movie {i}")),
                release_year: Some("2024".into()),
                rating: Some("4".into()),
                watched_at: Some("2024-06-01".into()),
                external_metadata_id: None,
                director: None,
                comment: None,
            }),
            is_duplicate: false,
        })
        .collect();

    let mut session = ImportSession::new(UserId::from_uuid(uid));
    let sid = session.id.clone();
    session.row_results = Some(rows);
    sessions.create(&session).await.unwrap();

    let confirmed_indices: Vec<usize> = (0..15).collect();
    let result = execute::execute(
        Arc::clone(&sessions) as _,
        Arc::new(NoopReviewLogger),
        ExecuteImportCommand {
            user_id: uid,
            session_id: sid.value(),
            confirmed_indices,
        },
    )
    .await
    .unwrap();

    assert_eq!(result.imported, 15);
    assert_eq!(result.skipped_duplicates, 0);
    assert!(result.failed.is_empty());
}
