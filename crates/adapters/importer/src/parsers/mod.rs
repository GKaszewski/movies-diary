mod csv;
mod json;
#[cfg(feature = "xlsx")]
mod xlsx;

pub use csv::parse_csv;
pub use json::parse_json;
#[cfg(feature = "xlsx")]
pub use xlsx::parse_xlsx;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
