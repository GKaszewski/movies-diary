use std::sync::Arc;

use domain::{
    errors::DomainError,
    ports::{DiaryExporter, DiaryRepository},
};

use crate::commands::ExportCommand;

pub struct ExportDiary {
    repository: Arc<dyn DiaryRepository>,
    exporter: Arc<dyn DiaryExporter>,
}

impl ExportDiary {
    pub async fn execute(&self, req: ExportCommand) -> Result<Vec<u8>, DomainError> {
        // 1. fetch all diary entries for the user
        // 2. delegate serialization to the port (exporter)

        // Return bytes of the exported diary, which can be written to a file or returned in an HTTP response
        Ok(vec![])
    }
}
