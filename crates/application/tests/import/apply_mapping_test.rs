use crate::support::{apply_import_mapping_command, create_import_session_command};
use domain::testing::{test_movie_with_external_id, test_movie_with_year};
use std::sync::Arc;

use uuid::Uuid;

use domain::{
    models::{
        AnnotatedRow,
        import::{ImportRow, ParsedFile, RowResult},
    },
    ports::{DocumentParser, MovieCommand},
    testing::InMemoryStore,
    value_objects::ExternalMetadataId,
};

use crate::support::TestContextBuilder;
use application::import::deps::{ApplyMappingDeps, CreateSessionDeps};
use application::import::{apply_mapping, create_session};

#[tokio::test]
async fn applies_mapping_to_session() {
    let sessions = InMemoryStore::new();
    let b = TestContextBuilder::new();
    let user_id = Uuid::new_v4();

    let create_deps = CreateSessionDeps {
        import_session: Arc::clone(&sessions) as _,
        document_parser: b.document_parser.clone(),
    };

    let session = create_session::execute(
        &create_deps,
        create_import_session_command(user_id, b"title\nTest"),
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
        apply_import_mapping_command(user_id, session.session_id.value()),
    )
    .await
    .unwrap();

    assert!(!rows.is_empty());
}

#[tokio::test]
async fn fails_when_session_not_found() {
    let sessions = InMemoryStore::new();
    let b = TestContextBuilder::new();

    let deps = ApplyMappingDeps {
        import_session: Arc::clone(&sessions) as _,
        document_parser: b.document_parser.clone(),
        movie_query: b.movie_query.clone(),
    };

    let result = apply_mapping::execute(
        &deps,
        apply_import_mapping_command(Uuid::new_v4(), Uuid::new_v4()),
    )
    .await;

    assert!(result.is_err());
}

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
    let movies = InMemoryStore::new();
    let sessions = Arc::clone(&movies);

    let ext_id = ExternalMetadataId::new("tt1234567".into()).unwrap();
    let movie = test_movie_with_external_id("Known Movie", 2020, ext_id);
    movies.upsert_movie(&movie, &[]).await.unwrap();

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
        create_import_session_command(user_id, b"title\nKnown Movie"),
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
        apply_import_mapping_command(user_id, session.session_id.value()),
    )
    .await
    .unwrap();

    let has_dup = rows.iter().any(|r| r.is_duplicate);
    assert!(has_dup, "row with matching external_id should be duplicate");
}

#[tokio::test]
async fn marks_duplicate_by_title_and_year() {
    let movies = InMemoryStore::new();
    let sessions = Arc::clone(&movies);

    let movie = test_movie_with_year("Duplicate Film", 2022);
    movies.upsert_movie(&movie, &[]).await.unwrap();

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
        create_import_session_command(user_id, b"title\nDuplicate Film"),
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
        apply_import_mapping_command(user_id, session.session_id.value()),
    )
    .await
    .unwrap();

    let has_dup = rows.iter().any(|r| r.is_duplicate);
    assert!(has_dup, "row with matching title+year should be duplicate");
}
