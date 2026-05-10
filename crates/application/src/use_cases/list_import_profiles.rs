use domain::{errors::DomainError, models::ImportProfile, value_objects::UserId};
use crate::context::AppContext;

pub async fn execute(ctx: &AppContext, user_id: &UserId) -> Result<Vec<ImportProfile>, DomainError> {
    ctx.import_profile_repository.list_for_user(user_id).await
}
