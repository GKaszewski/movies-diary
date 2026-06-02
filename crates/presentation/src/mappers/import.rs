use domain::models::AnnotatedRow;
use domain::models::import::RowResult;
use template_askama::{ImportPreviewRow, ImportRowStatus};

pub fn annotated_to_preview_row(idx: usize, annotated: &AnnotatedRow) -> ImportPreviewRow {
    match &annotated.result {
        RowResult::Valid(row) => {
            let cells = vec![
                row.title.clone().unwrap_or_default(),
                row.release_year.clone().unwrap_or_default(),
                row.director.clone().unwrap_or_default(),
                row.rating.clone().unwrap_or_default(),
                row.watched_at.clone().unwrap_or_default(),
                row.comment.clone().unwrap_or_default(),
            ];
            ImportPreviewRow {
                index: idx,
                status: if annotated.is_duplicate {
                    ImportRowStatus::Duplicate
                } else {
                    ImportRowStatus::Valid
                },
                cells,
            }
        }
        RowResult::Invalid { errors, raw } => ImportPreviewRow {
            index: idx,
            status: ImportRowStatus::Invalid(errors.join("; ")),
            cells: raw.iter().map(|(_, v)| v.clone()).collect(),
        },
    }
}
