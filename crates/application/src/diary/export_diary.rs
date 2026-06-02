use domain::{errors::DomainError, value_objects::UserId};

use crate::{context::AppContext, diary::queries::ExportQuery};

pub async fn execute(ctx: &AppContext, query: ExportQuery) -> Result<Vec<u8>, DomainError> {
    let entries = ctx
        .repos
        .diary
        .get_user_history(&UserId::from_uuid(query.user_id))
        .await?;
    ctx.services
        .diary_exporter
        .serialize_entries(&entries, query.format)
        .await
}
