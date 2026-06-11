use std::sync::Arc;

use chrono::Utc;
use domain::errors::DomainError;
use domain::events::DomainEvent;
use domain::models::wrapup::{DateRange, WrapUpStatus};
use domain::ports::{EventPublisher, WrapUpRepository};
use domain::value_objects::{UserId, WrapUpId};

use crate::wrapup::commands::RequestWrapUpCommand;

pub async fn execute(
    wrapup_repo: Arc<dyn WrapUpRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    cmd: RequestWrapUpCommand,
) -> Result<WrapUpId, DomainError> {
    let date_range = DateRange::new(cmd.start_date, cmd.end_date)?;

    if cmd.end_date > Utc::now().date_naive() {
        return Err(DomainError::ValidationError(
            "end_date cannot be in the future".into(),
        ));
    }

    let existing = wrapup_repo
        .find_existing(cmd.user_id, date_range.start(), date_range.end())
        .await?;

    if let Some(ref rec) = existing {
        match rec.status {
            WrapUpStatus::Ready | WrapUpStatus::Generating => return Ok(rec.id.clone()),
            WrapUpStatus::Failed => {
                wrapup_repo.delete(&rec.id).await?;
            }
            WrapUpStatus::Pending => return Ok(rec.id.clone()),
        }
    }

    let id = WrapUpId::generate();
    let record = domain::models::wrapup::WrapUpRecord {
        id: id.clone(),
        user_id: cmd.user_id,
        start_date: date_range.start(),
        end_date: date_range.end(),
        status: WrapUpStatus::Pending,
        report: None,
        error_message: None,
        created_at: Utc::now().naive_utc(),
        completed_at: None,
    };
    wrapup_repo.create(&record).await?;

    event_publisher
        .publish(&DomainEvent::WrapUpRequested {
            wrapup_id: id.clone(),
            user_id: cmd.user_id.map(UserId::from_uuid),
            start_date: date_range.start(),
            end_date: date_range.end(),
        })
        .await?;

    Ok(id)
}

#[cfg(test)]
#[path = "tests/generate.rs"]
mod tests;
