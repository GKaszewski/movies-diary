use crate::context::AppContext;
use crate::wrapup::{compute, queries::ComputeWrapUpQuery, storage::WrapUpStorage};
use domain::errors::DomainError;
use domain::events::DomainEvent;
use domain::models::wrapup::{DateRange, WrapUpScope, WrapUpStatus};
use domain::ports::VideoRenderAssets;
use domain::value_objects::WrapUpId;

pub async fn execute(
    ctx: &AppContext,
    wrapup_id: WrapUpId,
    user_id: Option<uuid::Uuid>,
    start_date: chrono::NaiveDate,
    end_date: chrono::NaiveDate,
) -> Result<(), DomainError> {
    if let Ok(Some(rec)) = ctx.repos.wrapup_repo.get_by_id(&wrapup_id).await
        && (rec.status == WrapUpStatus::Ready || rec.status == WrapUpStatus::Generating)
    {
        tracing::debug!(
            "wrapup {} already {:?}, skipping",
            wrapup_id.value(),
            rec.status
        );
        return Ok(());
    }

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
        date_range: DateRange::new(start_date, end_date)?,
    };

    match compute::execute(ctx, query).await {
        Ok(report) => {
            ctx.repos
                .wrapup_repo
                .set_complete(&wrapup_id, &report)
                .await?;

            if let Some(ref renderer) = ctx.services.video_renderer {
                let asset_storage = WrapUpStorage::new(ctx.services.image_storage.clone());
                let poster_images = asset_storage.resolve_poster_images(&report.poster_paths).await;
                let cast_images = asset_storage
                    .resolve_cast_images(&report.top_cast_profile_paths)
                    .await;
                let assets = VideoRenderAssets {
                    poster_images,
                    cast_images,
                };
                match renderer.render(&report, assets).await {
                    Ok(video_bytes) => {
                        if let Err(e) = asset_storage.store_video(&wrapup_id, &video_bytes).await {
                            tracing::warn!("failed to store wrapup video: {e}");
                        }
                    }
                    Err(e) => {
                        tracing::warn!("video render failed (non-fatal): {e}");
                    }
                }
            }

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
