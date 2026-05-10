mod mapper;
mod parsers;

use domain::{
    models::{AnnotatedRow, FieldMapping, FileFormat, ImportError, ParsedFile},
    ports::DocumentParser,
};

pub struct ImporterDocumentParser;

impl DocumentParser for ImporterDocumentParser {
    fn parse(&self, bytes: &[u8], format: FileFormat) -> Result<ParsedFile, ImportError> {
        match format {
            FileFormat::Csv => parsers::parse_csv(bytes),
            FileFormat::Json => parsers::parse_json(bytes),
            FileFormat::Xlsx => {
                #[cfg(feature = "xlsx")]
                { parsers::parse_xlsx(bytes) }
                #[cfg(not(feature = "xlsx"))]
                { Err(ImportError::Xlsx("XLSX support not compiled in".into())) }
            }
        }
    }

    fn apply_mapping(&self, file: &ParsedFile, mappings: &[FieldMapping]) -> Vec<AnnotatedRow> {
        mapper::apply_mapping(file, mappings)
    }
}
