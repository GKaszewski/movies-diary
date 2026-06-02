mod ffmpeg;
mod slides;

use async_trait::async_trait;
use domain::errors::DomainError;
use domain::models::wrapup::WrapUpReport;
use domain::ports::{VideoRenderAssets, VideoRenderConfig, WrapUpVideoRenderer};

#[derive(Default)]
pub struct FfmpegWrapUpRenderer;

impl FfmpegWrapUpRenderer {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl WrapUpVideoRenderer for FfmpegWrapUpRenderer {
    async fn render(
        &self,
        report: &WrapUpReport,
        assets: VideoRenderAssets,
        config: &VideoRenderConfig,
    ) -> Result<Vec<u8>, DomainError> {
        let (width, height) = config.resolution;

        let renderer = slides::SlideRenderer::new(
            config.font_path.as_deref(),
            config.logo_path.as_deref(),
            config.bg_dir.as_deref(),
        )?;

        let mut slide_pngs = Vec::new();
        slide_pngs.push(renderer.render_hero(report, width, height)?);
        slide_pngs.push(renderer.render_ratings(report, width, height)?);
        if !report.top_directors.is_empty() {
            slide_pngs.push(renderer.render_directors(
                report,
                &assets.cast_images,
                width,
                height,
            )?);
        }
        if !report.top_actors.is_empty() {
            slide_pngs.push(renderer.render_actors(report, &assets.cast_images, width, height)?);
        }
        if !report.top_genres.is_empty() {
            slide_pngs.push(renderer.render_genres(report, width, height)?);
        }
        slide_pngs.push(renderer.render_highlights(
            report,
            &assets.poster_images,
            width,
            height,
        )?);
        if !assets.poster_images.is_empty() {
            slide_pngs.push(renderer.render_mosaic(&assets.poster_images, width, height)?);
        } else {
            tracing::warn!("no poster images resolved, skipping mosaic slide");
        }

        ffmpeg::stitch_slides(&slide_pngs, config).await
    }
}
