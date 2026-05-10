use domain::models::{ImportError, ParsedFile};

pub fn parse_csv(bytes: &[u8]) -> Result<ParsedFile, ImportError> {
    if bytes.is_empty() {
        return Err(ImportError::Empty);
    }

    let delimiter = detect_delimiter(bytes);
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .from_reader(bytes);

    let columns: Vec<String> = rdr
        .headers()
        .map_err(|e| ImportError::Csv(e.to_string()))?
        .iter()
        .map(|s| s.trim().to_string())
        .collect();

    if columns.is_empty() {
        return Err(ImportError::NoHeader);
    }

    let rows: Vec<Vec<String>> = rdr
        .records()
        .map(|r| {
            r.map_err(|e| ImportError::Csv(e.to_string()))
                .map(|rec| {
                    let mut cells: Vec<String> = rec.iter().map(|f| f.trim().to_string()).collect();
                    cells.resize(columns.len(), String::new());
                    cells.truncate(columns.len());
                    cells
                })
        })
        .collect::<Result<_, _>>()?;

    if rows.is_empty() {
        return Err(ImportError::Empty);
    }

    Ok(ParsedFile { columns, rows })
}

fn detect_delimiter(bytes: &[u8]) -> u8 {
    let first_line = bytes.split(|&b| b == b'\n').next().unwrap_or(bytes);
    let tabs = first_line.iter().filter(|&&b| b == b'\t').count();
    let commas = first_line.iter().filter(|&&b| b == b',').count();
    if tabs > commas { b'\t' } else { b',' }
}
