use std::sync::Arc;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    ports::{EventHandler, ImageRefCommand, ImageStorage},
};

use crate::Format;

pub struct ImageConversionHandler {
    storage: Arc<dyn ImageStorage>,
    image_ref: Arc<dyn ImageRefCommand>,
    format: Format,
}

impl ImageConversionHandler {
    pub fn new(
        storage: Arc<dyn ImageStorage>,
        image_ref: Arc<dyn ImageRefCommand>,
        format: Format,
    ) -> Self {
        Self { storage, image_ref, format }
    }
}

#[async_trait]
impl EventHandler for ImageConversionHandler {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError> {
        let key = match event {
            DomainEvent::ImageStored { key } => key.clone(),
            _ => return Ok(()),
        };

        if key.ends_with(".avif") || key.ends_with(".webp") {
            return Ok(());
        }

        let bytes = self.storage.get(&key).await?;
        let format = self.format;

        let converted = tokio::task::spawn_blocking(move || convert(bytes, format))
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?
            .map_err(|e| DomainError::InfrastructureError(e))?;

        let ext = format.extension();
        let new_key = format!("{key}{ext}");
        self.storage.store(&new_key, &converted).await?;

        if let Err(e) = self.image_ref.swap(&key, &new_key).await {
            tracing::error!("swap failed for {key} → {new_key}: {e}");
            return Err(e);
        }

        if let Err(e) = self.storage.delete(&key).await {
            tracing::warn!("failed to delete old image key {key}: {e}");
        }

        tracing::info!("converted {key} → {new_key}");
        Ok(())
    }
}

fn convert(bytes: Vec<u8>, format: Format) -> Result<Vec<u8>, String> {
    let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;

    match format {
        Format::Avif => {
            let rgba = img.to_rgba8();
            let width = rgba.width() as usize;
            let height = rgba.height() as usize;
            let pixels: Vec<ravif::RGBA8> = rgba
                .pixels()
                .map(|p| ravif::RGBA8 { r: p.0[0], g: p.0[1], b: p.0[2], a: p.0[3] })
                .collect();
            let result = ravif::Encoder::new()
                .with_quality(80.0)
                .with_speed(6)
                .encode_rgba(ravif::Img::new(&pixels, width, height))
                .map_err(|e| e.to_string())?;
            Ok(result.avif_file.to_vec())
        }
        Format::Webp => {
            let rgba = img.to_rgba8();
            let (width, height) = (rgba.width(), rgba.height());
            let encoder = webp::Encoder::from_rgba(rgba.as_raw(), width, height);
            Ok(encoder.encode(80.0).to_vec())
        }
    }
}

#[cfg(test)]
#[path = "tests/handler.rs"]
mod tests;
