use domain::errors::DomainError;
use domain::value_objects::WrapUpId;

use crate::context::AppContext;
use crate::wrapup::storage::WrapUpStorage;

pub async fn execute(ctx: &AppContext, id: WrapUpId) -> Result<(), DomainError> {
    ctx.repos
        .wrapup_repo
        .get_by_id(&id)
        .await?
        .ok_or_else(|| DomainError::NotFound("wrap-up not found".into()))?;

    let storage = WrapUpStorage::new(ctx.services.image_storage.clone());
    let _ = storage.delete_video(&id).await;

    ctx.repos.wrapup_repo.delete(&id).await
}
