use bytes::Bytes;
use domain::{
    errors::DomainError,
    models::{DiaryEntry, ExportFormat},
    ports::DiaryExporter,
};
use futures::stream::BoxStream;

pub struct ExportAdapter;

impl DiaryExporter for ExportAdapter {
    fn stream_entries(
        &self,
        stream: BoxStream<'static, Result<DiaryEntry, DomainError>>,
        format: ExportFormat,
    ) -> BoxStream<'static, Result<Bytes, DomainError>> {
        match format {
            ExportFormat::Csv => stream_csv(stream),
            ExportFormat::Json => stream_json(stream),
        }
    }
}

fn stream_csv(
    entries: BoxStream<'static, Result<DiaryEntry, DomainError>>,
) -> BoxStream<'static, Result<Bytes, DomainError>> {
    use futures::StreamExt;
    let header = futures::stream::once(async {
        Ok(Bytes::from_static(
            b"title,year,director,rating,comment,watched_at,external_metadata_id\n",
        ))
    });
    let rows = entries.map(|r| r.map(|e| Bytes::from(csv_row(&e))));
    Box::pin(header.chain(rows))
}

fn stream_json(
    stream: BoxStream<'static, Result<DiaryEntry, DomainError>>,
) -> BoxStream<'static, Result<Bytes, DomainError>> {
    Box::pin(async_stream::stream! {
        futures::pin_mut!(stream);
        let mut is_first = true;
        while let Some(r) = futures::StreamExt::next(&mut stream).await {
            match r {
                Err(e) => { yield Err(e); return; }
                Ok(entry) => {
                    let json = serde_json::to_string(&entry_to_json(&entry))
                        .map_err(|e| DomainError::InfrastructureError(e.to_string()));
                    let json = match json {
                        Ok(s) => s,
                        Err(e) => { yield Err(e); return; }
                    };
                    let prefix = if is_first { "[" } else { "," };
                    is_first = false;
                    yield Ok(Bytes::from(format!("{}{}", prefix, json)));
                }
            }
        }
        if is_first {
            yield Ok(Bytes::from_static(b"[]"));
        } else {
            yield Ok(Bytes::from_static(b"]"));
        }
    })
}

fn csv_row(e: &DiaryEntry) -> String {
    let title = csv_escape(e.movie().title().value());
    let year = e.movie().release_year().value();
    let director = e.movie().director().map(csv_escape).unwrap_or_default();
    let rating = e.review().rating().value();
    let comment = e
        .review()
        .comment()
        .map(|c| csv_escape(c.value()))
        .unwrap_or_default();
    let watched_at = e.review().watched_at().format("%Y-%m-%d");
    let ext_id = e
        .movie()
        .external_metadata_id()
        .map(|id| id.value().to_string())
        .unwrap_or_default();
    format!(
        "{},{},{},{},{},{},{}\n",
        title, year, director, rating, comment, watched_at, ext_id
    )
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn entry_to_json(e: &DiaryEntry) -> serde_json::Value {
    serde_json::json!({
        "title": e.movie().title().value(),
        "year": e.movie().release_year().value(),
        "director": e.movie().director(),
        "rating": e.review().rating().value(),
        "comment": e.review().comment().map(|c| c.value().to_string()),
        "watched_at": e.review().watched_at().format("%Y-%m-%d").to_string(),
        "external_metadata_id": e.movie().external_metadata_id().map(|id| id.value().to_string()),
    })
}

#[cfg(test)]
#[path = "tests/lib.rs"]
mod tests;
