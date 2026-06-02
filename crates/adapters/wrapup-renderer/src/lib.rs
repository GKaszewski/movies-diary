mod charts;
mod ffmpeg;
mod slides;

use async_trait::async_trait;
use domain::errors::DomainError;
use domain::models::wrapup::WrapUpReport;
use domain::ports::{VideoRenderConfig, WrapUpVideoRenderer};

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
        poster_images: Vec<(String, Vec<u8>)>,
        config: &VideoRenderConfig,
    ) -> Result<Vec<u8>, DomainError> {
        let (width, height) = config.resolution;

        let renderer =
            slides::SlideRenderer::new(config.font_path.as_deref(), config.logo_path.as_deref())?;

        // 1. Generate slide images
        let mut slide_pngs = Vec::new();
        slide_pngs.push(renderer.render_hero(report, width, height)?);
        slide_pngs.push(renderer.render_ratings(report, width, height)?);
        if !report.top_directors.is_empty() {
            slide_pngs.push(renderer.render_directors(report, width, height)?);
        }
        if !report.top_actors.is_empty() {
            slide_pngs.push(renderer.render_actors(report, width, height)?);
        }
        if !report.top_genres.is_empty() {
            slide_pngs.push(charts::render_genre_chart(report, width, height)?);
        }
        slide_pngs.push(renderer.render_highlights(report, width, height)?);
        if !poster_images.is_empty() {
            slide_pngs.push(renderer.render_mosaic(&poster_images, width, height)?);
        }

        // 2. Stitch into video
        ffmpeg::stitch_slides(&slide_pngs, config).await
    }
}
