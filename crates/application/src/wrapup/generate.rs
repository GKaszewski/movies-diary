use chrono::Utc;
use domain::errors::DomainError;
use domain::events::DomainEvent;
use domain::models::wrapup::WrapUpStatus;
use domain::value_objects::{UserId, WrapUpId};

use crate::context::AppContext;
use crate::wrapup::commands::RequestWrapUpCommand;

pub async fn execute(ctx: &AppContext, cmd: RequestWrapUpCommand) -> Result<WrapUpId, DomainError> {
    if cmd.end_date <= cmd.start_date {
        return Err(DomainError::ValidationError(
            "end_date must be after start_date".into(),
        ));
    }
    let days = (cmd.end_date - cmd.start_date).num_days();
    if days > 366 {
        return Err(DomainError::ValidationError(
            "date range cannot exceed 366 days".into(),
        ));
    }
    if cmd.end_date > Utc::now().date_naive() {
        return Err(DomainError::ValidationError(
            "end_date cannot be in the future".into(),
        ));
    }

    let existing = ctx
        .repos
        .wrapup_repo
        .find_existing(cmd.user_id, cmd.start_date, cmd.end_date)
        .await?;

    if let Some(ref rec) = existing {
        match rec.status {
            WrapUpStatus::Ready | WrapUpStatus::Generating => return Ok(rec.id.clone()),
            WrapUpStatus::Failed => {
                ctx.repos.wrapup_repo.delete(&rec.id).await?;
            }
            WrapUpStatus::Pending => return Ok(rec.id.clone()),
        }
    }

    let id = WrapUpId::generate();
    let record = domain::models::wrapup::WrapUpRecord {
        id: id.clone(),
        user_id: cmd.user_id,
        start_date: cmd.start_date,
        end_date: cmd.end_date,
        status: WrapUpStatus::Pending,
        report_json: None,
        error_message: None,
        created_at: Utc::now().naive_utc(),
        completed_at: None,
    };
    ctx.repos.wrapup_repo.create(&record).await?;

    ctx.services
        .event_publisher
        .publish(&DomainEvent::WrapUpRequested {
            wrapup_id: id.clone(),
            user_id: cmd.user_id.map(UserId::from_uuid),
            start_date: cmd.start_date,
            end_date: cmd.end_date,
        })
        .await?;

    Ok(id)
}
