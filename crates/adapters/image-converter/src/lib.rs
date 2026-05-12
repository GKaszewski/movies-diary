mod backfill;
mod config;
mod handler;

pub use backfill::ConversionBackfillJob;
pub use config::{ConversionConfig, Format};
pub use handler::ImageConversionHandler;

use std::sync::Arc;
use domain::ports::{EventHandler, EventPublisher, ImageRefPort, ImageStorage, PeriodicJob};

pub fn build(
    image_storage: Arc<dyn ImageStorage>,
    image_ref: Arc<dyn ImageRefPort>,
    event_publisher: Arc<dyn EventPublisher>,
) -> anyhow::Result<Option<(Arc<dyn EventHandler>, Arc<dyn PeriodicJob>)>> {
    let config = match ConversionConfig::from_env()? {
        Some(c) => c,
        None => return Ok(None),
    };

    let format = config.format;

    let handler = Arc::new(ImageConversionHandler::new(
        Arc::clone(&image_storage),
        Arc::clone(&image_ref),
        format,
    )) as Arc<dyn EventHandler>;

    let job = Arc::new(ConversionBackfillJob::new(
        Arc::clone(&image_ref),
        Arc::clone(&event_publisher),
    )) as Arc<dyn PeriodicJob>;

    Ok(Some((handler, job)))
}
