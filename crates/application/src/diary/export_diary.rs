use std::sync::Arc;

use domain::{
    errors::DomainError,
    ports::{DiaryExporter, DiaryRepository},
    value_objects::UserId,
};

use crate::diary::queries::ExportQuery;

pub async fn execute(
    diary: &Arc<dyn DiaryRepository>,
    diary_exporter: &Arc<dyn DiaryExporter>,
    query: ExportQuery,
) -> Result<Vec<u8>, DomainError> {
    let entries = diary
        .get_user_history(&UserId::from_uuid(query.user_id))
        .await?;
    diary_exporter.serialize_entries(&entries, query.format).await
}
