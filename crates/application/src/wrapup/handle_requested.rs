use crate::context::AppContext;
use crate::wrapup::{compute, queries::ComputeWrapUpQuery};
use domain::errors::DomainError;
use domain::events::DomainEvent;
use domain::models::wrapup::{DateRange, WrapUpReport, WrapUpScope, WrapUpStatus};
use domain::ports::VideoRenderConfig;
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

            // Optionally render video (non-fatal)
            if let Some(ref renderer) = ctx.services.video_renderer {
                let poster_images = resolve_poster_images(ctx, &report).await;
                let config = VideoRenderConfig {
                    slide_duration_secs: 4,
                    transition_duration_secs: 0.8,
                    resolution: (1080, 1920),
                    ffmpeg_path: "ffmpeg".to_string(),
                };
                match renderer.render(&report, poster_images, &config).await {
                    Ok(video_bytes) => {
                        let video_key = format!("wrapups/{}/video.mp4", wrapup_id.value());
                        if let Err(e) = ctx.services.image_storage.store(&video_key, &video_bytes).await {
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

async fn resolve_poster_images(ctx: &AppContext, report: &WrapUpReport) -> Vec<(String, Vec<u8>)> {
    let mut images = Vec::new();
    for path in report.poster_paths.iter().take(20) {
        match ctx.services.image_storage.get(path).await {
            Ok(bytes) => images.push((path.clone(), bytes)),
            Err(_) => {}
        }
    }
    images
}
