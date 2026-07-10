use std::sync::Arc;

use chrono::NaiveDateTime;
use domain::{
    errors::DomainError,
    models::{ImportRow, import::RowResult},
    value_objects::{ImportSessionId, UserId},
};
use uuid::Uuid;

use crate::{
    diary::commands::{LogReviewCommand, MovieInput},
    import::commands::ExecuteImportCommand,
};

use super::deps::ExecuteImportDeps;

const CONCURRENCY_LIMIT: usize = 10;

pub struct ImportSummary {
    pub imported: usize,
    pub skipped_duplicates: usize,
    pub failed: Vec<(usize, String)>,
}

pub async fn execute(
    deps: &ExecuteImportDeps,
    cmd: ExecuteImportCommand,
) -> Result<ImportSummary, DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    let session_id = ImportSessionId::from_uuid(cmd.session_id);
    let confirmed_indices = cmd.confirmed_indices;
    let session = deps
        .import_session
        .get(&session_id, &user_id)
        .await?
        .ok_or_else(|| DomainError::NotFound("import session".into()))?;

    let row_results = session.row_results.unwrap_or_default();
    let confirmed_set: std::collections::HashSet<usize> = confirmed_indices.into_iter().collect();

    let mut imported = 0;
    let mut skipped_duplicates = 0;
    let mut failed = Vec::new();

    let semaphore = Arc::new(tokio::sync::Semaphore::new(CONCURRENCY_LIMIT));
    let mut tasks: tokio::task::JoinSet<(usize, Result<(), String>)> = tokio::task::JoinSet::new();

    for (idx, annotated) in row_results.into_iter().enumerate() {
        if !confirmed_set.contains(&idx) {
            skipped_duplicates += 1;
            continue;
        }
        match annotated.result {
            RowResult::Invalid { errors, .. } => {
                failed.push((idx, errors.join("; ")));
            }
            RowResult::Valid(row) => match row_to_command(&row, user_id.value()) {
                Err(e) => failed.push((idx, e)),
                Ok(log_cmd) => {
                    let permit = Arc::clone(&semaphore).acquire_owned().await.unwrap();
                    let logger = deps.review_logger.clone();
                    tasks.spawn(async move {
                        let result = logger.log_review(log_cmd).await.map_err(|e| e.to_string());
                        drop(permit);
                        (idx, result)
                    });
                }
            },
        }
    }

    while let Some(res) = tasks.join_next().await {
        let (idx, outcome) = res.expect("import task panicked");
        match outcome {
            Ok(()) => imported += 1,
            Err(e) => failed.push((idx, e)),
        }
    }

    deps.import_session.delete(&session_id).await?;

    Ok(ImportSummary {
        imported,
        skipped_duplicates,
        failed,
    })
}

#[cfg(test)]
#[path = "tests/execute.rs"]
mod tests;

fn row_to_command(row: &ImportRow, user_id: Uuid) -> Result<LogReviewCommand, String> {
    let rating = row
        .rating
        .as_deref()
        .ok_or("missing rating")?
        .parse::<u8>()
        .map_err(|_| "rating is not a valid u8".to_string())?;

    let watched_at_str = row.watched_at.as_deref().ok_or("missing watched_at")?;
    let watched_at =
        NaiveDateTime::parse_from_str(&format!("{} 00:00:00", watched_at_str), "%Y-%m-%d %H:%M:%S")
            .or_else(|_| NaiveDateTime::parse_from_str(watched_at_str, "%Y-%m-%d %H:%M:%S"))
            .or_else(|_| NaiveDateTime::parse_from_str(watched_at_str, "%Y-%m-%dT%H:%M:%S"))
            .map_err(|_| format!("cannot parse watched_at: '{}'", watched_at_str))?;

    Ok(LogReviewCommand {
        user_id,
        input: MovieInput {
            movie_id: None,
            external_metadata_id: row.external_metadata_id.clone(),
            manual_title: row.title.clone(),
            manual_release_year: row.release_year.as_deref().and_then(|s| s.parse().ok()),
            manual_director: row.director.clone(),
        },
        rating,
        comment: row.comment.clone(),
        watched_at,
        watch_medium: None,
    })
}
