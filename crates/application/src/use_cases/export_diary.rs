use domain::{errors::DomainError, value_objects::UserId};

use crate::{commands::ExportCommand, context::AppContext};

pub async fn execute(ctx: &AppContext, cmd: ExportCommand) -> Result<Vec<u8>, DomainError> {
    let entries = ctx
        .diary_repository
        .get_user_history(&UserId::from_uuid(cmd.user_id))
        .await?;
    ctx.diary_exporter
        .serialize_entries(&entries, cmd.format)
        .await
}
