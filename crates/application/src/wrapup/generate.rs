use domain::errors::DomainError;
use domain::events::DomainEvent;
use domain::models::wrapup::WrapUpStatus;
use domain::value_objects::{UserId, WrapUpId};

use crate::context::AppContext;
use crate::wrapup::commands::RequestWrapUpCommand;

pub async fn execute(ctx: &AppContext, cmd: RequestWrapUpCommand) -> Result<WrapUpId, DomainError> {
    let existing = ctx
        .repos
        .wrapup_repo
        .find_existing(cmd.user_id, cmd.start_date, cmd.end_date)
        .await?;

    if let Some(ref rec) = existing {
        if rec.status == WrapUpStatus::Ready || rec.status == WrapUpStatus::Generating {
            return Ok(rec.id.clone());
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
        created_at: chrono::Utc::now().naive_utc(),
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
