use calamine::{Data, Reader, Xlsx, open_workbook_from_rs};
use domain::models::{ImportError, ParsedFile};
use std::io::Cursor;

pub fn parse_xlsx(bytes: &[u8]) -> Result<ParsedFile, ImportError> {
    let cursor = Cursor::new(bytes);
    let mut workbook: Xlsx<_> = open_workbook_from_rs(cursor)
        .map_err(|e: calamine::XlsxError| ImportError::Xlsx(e.to_string()))?;

    let sheet_name = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or(ImportError::Empty)?;

    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|e| ImportError::Xlsx(e.to_string()))?;

    let mut iter = range.rows();

    let header = iter.next().ok_or(ImportError::NoHeader)?;
    let columns: Vec<String> = header
        .iter()
        .map(|c| cell_to_string(c).trim().to_string())
        .collect();

    if columns.is_empty() {
        return Err(ImportError::NoHeader);
    }

    let rows: Vec<Vec<String>> = iter
        .map(|row| {
            let mut cells: Vec<String> = row.iter().map(cell_to_string).collect();
            cells.resize(columns.len(), String::new());
            cells.truncate(columns.len());
            cells
        })
        .collect();

    if rows.is_empty() {
        return Err(ImportError::Empty);
    }

    Ok(ParsedFile { columns, rows })
}

fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::String(s) => s.clone(),
        Data::Float(f) => {
            if f.fract() == 0.0 {
                format!("{}", *f as i64)
            } else {
                format!("{}", f)
            }
        }
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(dt) => {
            // ExcelDateTime::to_ymd_hms_milli() works without the chrono feature.
            let (year, month, day, _, _, _, _) = dt.to_ymd_hms_milli();
            format!("{:04}-{:02}-{:02}", year, month, day)
        }
        Data::DateTimeIso(s) => s.clone(),
        Data::DurationIso(s) => s.clone(),
        Data::Empty | Data::Error(_) => String::new(),
        // Fallback for unexpected calamine Data variants; renders as debug string
        other => format!("{other:?}"),
    }
}
