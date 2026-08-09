use bytes::Bytes;
use domain::{errors::DomainError, value_objects::UserId};
use futures::stream::BoxStream;

use crate::diary::deps::ExportDiaryDeps;
use crate::diary::queries::ExportQuery;

pub fn execute(
    deps: &ExportDiaryDeps,
    query: ExportQuery,
) -> BoxStream<'static, Result<Bytes, DomainError>> {
    let user_id = UserId::from_uuid(query.user_id);
    let entry_stream = deps.diary.stream_user_history(user_id);
    deps.diary_exporter
        .stream_entries(entry_stream, query.format)
}
