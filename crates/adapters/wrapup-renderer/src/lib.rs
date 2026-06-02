mod ffmpeg;
mod slides;

use async_trait::async_trait;
use domain::errors::DomainError;
use domain::models::wrapup::WrapUpReport;
use domain::ports::{VideoRenderAssets, WrapUpVideoRenderer};

pub struct RendererConfig {
    pub slide_duration_secs: u32,
    pub transition_duration_secs: f32,
    pub resolution: (u32, u32),
    pub ffmpeg_path: String,
    pub font_path: Option<String>,
    pub logo_path: Option<String>,
    pub bg_dir: Option<String>,
}

pub struct FfmpegWrapUpRenderer {
    config: RendererConfig,
    slide_renderer: slides::SlideRenderer,
}

impl FfmpegWrapUpRenderer {
    pub fn new(config: RendererConfig) -> Result<Self, DomainError> {
        let slide_renderer = slides::SlideRenderer::new(
            config.font_path.as_deref(),
            config.logo_path.as_deref(),
            config.bg_dir.as_deref(),
        )?;
        Ok(Self {
            config,
            slide_renderer,
        })
    }
}

#[async_trait]
impl WrapUpVideoRenderer for FfmpegWrapUpRenderer {
    async fn render(
        &self,
        report: &WrapUpReport,
        assets: VideoRenderAssets,
    ) -> Result<Vec<u8>, DomainError> {
        let (width, height) = self.config.resolution;

        let mut slide_pngs = Vec::new();
        slide_pngs.push(self.slide_renderer.render_hero(report, width, height)?);
        slide_pngs.push(self.slide_renderer.render_ratings(report, width, height)?);
        if !report.top_directors.is_empty() {
            slide_pngs.push(self.slide_renderer.render_directors(
                report,
                &assets.cast_images,
                width,
                height,
            )?);
        }
        if !report.top_actors.is_empty() {
            slide_pngs.push(
                self.slide_renderer
                    .render_actors(report, &assets.cast_images, width, height)?,
            );
        }
        if !report.top_genres.is_empty() {
            slide_pngs.push(self.slide_renderer.render_genres(report, width, height)?);
        }
        slide_pngs.push(self.slide_renderer.render_highlights(
            report,
            &assets.poster_images,
            width,
            height,
        )?);
        if !assets.poster_images.is_empty() {
            slide_pngs.push(
                self.slide_renderer
                    .render_mosaic(&assets.poster_images, width, height)?,
            );
        } else {
            tracing::warn!("no poster images resolved, skipping mosaic slide");
        }

        ffmpeg::stitch_slides(
            &slide_pngs,
            &self.config.ffmpeg_path,
            self.config.slide_duration_secs,
            self.config.resolution,
        )
        .await
    }
}
