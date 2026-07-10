use std::sync::Arc;

use uuid::Uuid;

use domain::{
    models::{
        AnnotatedRow, Movie,
        import::{ImportRow, ParsedFile, RowResult},
    },
    ports::{DocumentParser, MovieCommand},
    testing::{InMemoryImportSessionRepository, InMemoryMovieRepository},
    value_objects::{ExternalMetadataId, MovieTitle, ReleaseYear},
};

use crate::import::deps::{ApplyMappingDeps, CreateSessionDeps};
use crate::import::{
    apply_mapping,
    commands::{ApplyImportMappingCommand, CreateImportSessionCommand},
    create_session,
};
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn applies_mapping_to_session() {
    let sessions = InMemoryImportSessionRepository::new();
    let b = TestContextBuilder::new();
    let user_id = Uuid::new_v4();

    let create_deps = CreateSessionDeps {
        import_session: Arc::clone(&sessions) as _,
        document_parser: b.document_parser.clone(),
    };

    let session = create_session::execute(
        &create_deps,
        CreateImportSessionCommand {
            user_id,
            bytes: b"title\nTest".to_vec(),
            format: domain::models::FileFormat::Csv,
        },
    )
    .await
    .unwrap();

    let mapping_deps = ApplyMappingDeps {
        import_session: Arc::clone(&sessions) as _,
        document_parser: b.document_parser.clone(),
        movie_query: b.movie_query.clone(),
    };

    let rows = apply_mapping::execute(
        &mapping_deps,
        ApplyImportMappingCommand {
            user_id,
            session_id: session.session_id.value(),
            mappings: vec![],
        },
    )
    .await
    .unwrap();

    assert!(!rows.is_empty());
}

#[tokio::test]
async fn fails_when_session_not_found() {
    let sessions = InMemoryImportSessionRepository::new();
    let b = TestContextBuilder::new();

    let deps = ApplyMappingDeps {
        import_session: Arc::clone(&sessions) as _,
        document_parser: b.document_parser.clone(),
        movie_query: b.movie_query.clone(),
    };

    let result = apply_mapping::execute(
        &deps,
        ApplyImportMappingCommand {
            user_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            mappings: vec![],
        },
    )
    .await;

    assert!(result.is_err());
}

/// A document parser that returns rows with specific field values for testing
/// the mark_duplicates logic.
struct DuplicateTestParser {
    rows: Vec<ImportRow>,
}

impl DocumentParser for DuplicateTestParser {
    fn parse(
        &self,
        _: &[u8],
        _: domain::models::FileFormat,
    ) -> Result<ParsedFile, domain::models::import::ImportError> {
        Ok(ParsedFile {
            columns: vec!["title".into()],
            rows: vec![vec!["x".into()]],
        })
    }

    fn apply_mapping(
        &self,
        _: &ParsedFile,
        _: &[domain::models::FieldMapping],
    ) -> Vec<AnnotatedRow> {
        self.rows
            .iter()
            .map(|r| AnnotatedRow {
                result: RowResult::Valid(r.clone()),
                is_duplicate: false,
            })
            .collect()
    }
}

#[tokio::test]
async fn marks_duplicate_by_external_id() {
    let movies = InMemoryMovieRepository::new();
    let sessions = InMemoryImportSessionRepository::new();

    let ext_id = ExternalMetadataId::new("tt1234567".into()).unwrap();
    let movie = Movie::new(
        Some(ext_id),
        MovieTitle::new("Known Movie".into()).unwrap(),
        ReleaseYear::new(2020).unwrap(),
        None,
        None,
    );
    movies.upsert_movie(&movie).await.unwrap();

    let parser = Arc::new(DuplicateTestParser {
        rows: vec![ImportRow {
            title: Some("Known Movie".into()),
            release_year: Some("2020".into()),
            external_metadata_id: Some("tt1234567".into()),
            ..ImportRow::default()
        }],
    });

    let user_id = Uuid::new_v4();

    let create_deps = CreateSessionDeps {
        import_session: Arc::clone(&sessions) as _,
        document_parser: Arc::clone(&parser) as _,
    };

    let session = create_session::execute(
        &create_deps,
        CreateImportSessionCommand {
            user_id,
            bytes: b"title\nKnown Movie".to_vec(),
            format: domain::models::FileFormat::Csv,
        },
    )
    .await
    .unwrap();

    let mapping_deps = ApplyMappingDeps {
        import_session: Arc::clone(&sessions) as _,
        document_parser: Arc::clone(&parser) as _,
        movie_query: Arc::clone(&movies) as _,
    };

    let rows = apply_mapping::execute(
        &mapping_deps,
        ApplyImportMappingCommand {
            user_id,
            session_id: session.session_id.value(),
            mappings: vec![],
        },
    )
    .await
    .unwrap();

    let has_dup = rows.iter().any(|r| r.is_duplicate);
    assert!(has_dup, "row with matching external_id should be duplicate");
}

#[tokio::test]
async fn marks_duplicate_by_title_and_year() {
    let movies = InMemoryMovieRepository::new();
    let sessions = InMemoryImportSessionRepository::new();

    let movie = Movie::new(
        None,
        MovieTitle::new("Duplicate Film".into()).unwrap(),
        ReleaseYear::new(2022).unwrap(),
        None,
        None,
    );
    movies.upsert_movie(&movie).await.unwrap();

    let parser = Arc::new(DuplicateTestParser {
        rows: vec![ImportRow {
            title: Some("Duplicate Film".into()),
            release_year: Some("2022".into()),
            ..ImportRow::default()
        }],
    });

    let user_id = Uuid::new_v4();

    let create_deps = CreateSessionDeps {
        import_session: Arc::clone(&sessions) as _,
        document_parser: Arc::clone(&parser) as _,
    };

    let session = create_session::execute(
        &create_deps,
        CreateImportSessionCommand {
            user_id,
            bytes: b"title\nDuplicate Film".to_vec(),
            format: domain::models::FileFormat::Csv,
        },
    )
    .await
    .unwrap();

    let mapping_deps = ApplyMappingDeps {
        import_session: Arc::clone(&sessions) as _,
        document_parser: Arc::clone(&parser) as _,
        movie_query: Arc::clone(&movies) as _,
    };

    let rows = apply_mapping::execute(
        &mapping_deps,
        ApplyImportMappingCommand {
            user_id,
            session_id: session.session_id.value(),
            mappings: vec![],
        },
    )
    .await
    .unwrap();

    let has_dup = rows.iter().any(|r| r.is_duplicate);
    assert!(has_dup, "row with matching title+year should be duplicate");
}
