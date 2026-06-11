use std::sync::Arc;

use bytes::Bytes;
use domain::{
    errors::DomainError,
    ports::{DiaryExporter, DiaryRepository},
    value_objects::UserId,
};
use futures::stream::BoxStream;

use crate::diary::queries::ExportQuery;

pub fn execute(
    diary: &Arc<dyn DiaryRepository>,
    diary_exporter: &Arc<dyn DiaryExporter>,
    query: ExportQuery,
) -> BoxStream<'static, Result<Bytes, DomainError>> {
    let user_id = UserId::from_uuid(query.user_id);
    let entry_stream = diary.stream_user_history(user_id);
    diary_exporter.stream_entries(entry_stream, query.format)
}
