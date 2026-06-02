use crate::context::AppContext;
use crate::wrapup::{compute, queries::ComputeWrapUpQuery};
use domain::errors::DomainError;
use domain::events::DomainEvent;
use domain::models::wrapup::{DateRange, WrapUpScope, WrapUpStatus};
use domain::ports::{VideoRenderAssets, VideoRenderConfig};
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
        date_range: DateRange::new(start_date, end_date)?,
    };

    match compute::execute(ctx, query).await {
        Ok(report) => {
            let json = serde_json::to_string(&report)
                .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
            ctx.repos
                .wrapup_repo
                .set_complete(&wrapup_id, &json)
                .await?;

            if let Some(ref renderer) = ctx.services.video_renderer {
                let poster_images = resolve_images(ctx, &report.poster_paths, "poster").await;
                let cast_keys: Vec<String> = report
                    .top_cast_profile_paths
                    .iter()
                    .map(|p| format!("cast{p}"))
                    .collect();
                let cast_images = resolve_images(ctx, &cast_keys, "cast").await;
                let wc = &ctx.config.wrapup;
                let config = VideoRenderConfig {
                    slide_duration_secs: 4,
                    transition_duration_secs: 0.8,
                    resolution: (1080, 1920),
                    ffmpeg_path: wc.ffmpeg_path.clone(),
                    font_path: wc.font_path.clone(),
                    logo_path: wc.logo_path.clone(),
                    bg_dir: wc.bg_dir.clone(),
                };
                let assets = VideoRenderAssets {
                    poster_images,
                    cast_images,
                };
                match renderer.render(&report, assets, &config).await {
                    Ok(video_bytes) => {
                        let video_key = format!("wrapups/{}/video.mp4", wrapup_id.value());
                        if let Err(e) = ctx
                            .services
                            .image_storage
                            .store(&video_key, &video_bytes)
                            .await
                        {
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

async fn resolve_images(
    ctx: &AppContext,
    paths: &[String],
    label: &str,
) -> Vec<(String, Vec<u8>)> {
    let mut images = Vec::new();
    for path in paths.iter().take(20) {
        match ctx.services.image_storage.get(path).await {
            Ok(bytes) => images.push((path.clone(), bytes)),
            Err(e) => tracing::debug!("{label} fetch skipped for {path}: {e}"),
        }
    }
    tracing::info!("resolved {}/{} {label} images", images.len(), paths.len());
    images
}
