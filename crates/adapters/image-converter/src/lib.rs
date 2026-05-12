mod backfill;
mod config;
mod handler;

pub use backfill::ConversionBackfillJob;
pub use config::{ConversionConfig, Format};
pub use handler::ImageConversionHandler;

use std::sync::Arc;
use domain::ports::{EventHandler, EventPublisher, ImageRefCommand, ImageRefQuery, ImageStorage, PeriodicJob};

pub fn build(
    image_storage: Arc<dyn ImageStorage>,
    image_ref_command: Arc<dyn ImageRefCommand>,
    image_ref_query: Arc<dyn ImageRefQuery>,
    event_publisher: Arc<dyn EventPublisher>,
) -> anyhow::Result<Option<(Arc<dyn EventHandler>, Arc<dyn PeriodicJob>)>> {
    let config = match ConversionConfig::from_env()? {
        Some(c) => c,
        None => return Ok(None),
    };

    let format = config.format;

    let handler = Arc::new(ImageConversionHandler::new(
        Arc::clone(&image_storage),
        image_ref_command,
        format,
    )) as Arc<dyn EventHandler>;

    let job = Arc::new(ConversionBackfillJob::new(
        image_ref_query,
        Arc::clone(&event_publisher),
    )) as Arc<dyn PeriodicJob>;

    Ok(Some((handler, job)))
}
