use crate::context::AppContext;
use crate::wrapup::{compute, queries::ComputeWrapUpQuery};
use domain::errors::DomainError;
use domain::events::DomainEvent;
use domain::models::wrapup::{DateRange, WrapUpScope, WrapUpStatus};
use domain::value_objects::WrapUpId;

pub async fn execute(
    ctx: &AppContext,
    wrapup_id: WrapUpId,
    user_id: Option<uuid::Uuid>,
    start_date: chrono::NaiveDate,
    end_date: chrono::NaiveDate,
) -> Result<(), DomainError> {
    ctx.repos
        .wrapup_repo
        .update_status(&wrapup_id, &WrapUpStatus::Generating, None)
        .await?;

    let scope = match user_id {
        Some(uid) => WrapUpScope::User(uid),
        None => WrapUpScope::Global,
    };
    let query = ComputeWrapUpQuery {
        scope,
        date_range: DateRange {
            start: start_date,
            end: end_date,
        },
    };

    match compute::execute(ctx, query).await {
        Ok(report) => {
            let json = serde_json::to_string(&report)
                .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
            ctx.repos.wrapup_repo.set_complete(&wrapup_id, &json).await?;
            ctx.services
                .event_publisher
                .publish(&DomainEvent::WrapUpCompleted { wrapup_id })
                .await?;
            Ok(())
        }
        Err(e) => {
            ctx.repos
                .wrapup_repo
                .update_status(&wrapup_id, &WrapUpStatus::Failed, Some(&e.to_string()))
                .await?;
            Err(e)
        }
    }
}
