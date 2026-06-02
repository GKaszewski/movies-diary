use domain::errors::DomainError;
use domain::value_objects::WrapUpId;

use crate::context::AppContext;

pub async fn execute(ctx: &AppContext, id: WrapUpId) -> Result<(), DomainError> {
    let record = ctx
        .repos
        .wrapup_repo
        .get_by_id(&id)
        .await?
        .ok_or_else(|| DomainError::NotFound("wrap-up not found".into()))?;

    let wrapup_key = format!("wrapups/{}", id.value());
    let video_key = format!("{wrapup_key}/video.mp4");
    let _ = ctx.services.image_storage.delete(&video_key).await;

    ctx.repos.wrapup_repo.delete(&record.id).await
}
