use domain::models::{ImportError, ParsedFile};
use serde_json::Value;

pub fn parse_json(bytes: &[u8]) -> Result<ParsedFile, ImportError> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|e| ImportError::Json(e.to_string()))?;

    let arr = value
        .as_array()
        .ok_or_else(|| ImportError::Json("expected a JSON array".into()))?;

    if arr.is_empty() {
        return Err(ImportError::Empty);
    }

    let first = arr[0]
        .as_object()
        .ok_or_else(|| ImportError::Json("array elements must be objects".into()))?;
    let columns: Vec<String> = first.keys().cloned().collect();

    if columns.is_empty() {
        return Err(ImportError::NoHeader);
    }

    let rows: Vec<Vec<String>> = arr
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let obj = item.as_object().ok_or_else(|| {
                ImportError::Json(format!("element at index {} is not an object", idx))
            })?;
            Ok(columns
                .iter()
                .map(|col| obj.get(col).map(value_to_string).unwrap_or_default())
                .collect())
        })
        .collect::<Result<_, ImportError>>()?;

    Ok(ParsedFile { columns, rows })
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}
