use domain::{errors::DomainError, value_objects::UserId};

use crate::{context::AppContext, queries::ExportQuery};

pub async fn execute(ctx: &AppContext, query: ExportQuery) -> Result<Vec<u8>, DomainError> {
    let entries = ctx
        .diary_repository
        .get_user_history(&UserId::from_uuid(query.user_id))
        .await?;
    ctx.diary_exporter
        .serialize_entries(&entries, query.format)
        .await
}
