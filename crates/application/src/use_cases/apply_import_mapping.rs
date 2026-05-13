use domain::{
    errors::DomainError,
    models::{AnnotatedRow, import::RowResult},
    value_objects::{ExternalMetadataId, ImportSessionId, MovieTitle, ReleaseYear, UserId},
};

use crate::{commands::ApplyImportMappingCommand, context::AppContext};

pub async fn execute(
    ctx: &AppContext,
    cmd: ApplyImportMappingCommand,
) -> Result<Vec<AnnotatedRow>, DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    let session_id = ImportSessionId::from_uuid(cmd.session_id);
    let mappings = cmd.mappings;
    let mut session = ctx
        .import_session_repository
        .get(&session_id, &user_id)
        .await?
        .ok_or_else(|| DomainError::NotFound("import session".into()))?;

    // clone to avoid borrow conflict when mutating session fields below
    let parsed = session
        .parsed_file
        .clone()
        .ok_or_else(|| DomainError::ValidationError("session has no parsed file".into()))?;

    let mut annotated = ctx.document_parser.apply_mapping(&parsed, &mappings);

    for row in annotated.iter_mut() {
        if let RowResult::Valid(ref import_row) = row.result {
            row.is_duplicate = check_duplicate(ctx, import_row).await?;
        }
    }

    session.field_mappings = Some(mappings);
    session.row_results = Some(annotated.clone());

    ctx.import_session_repository.update(&session).await?;

    Ok(annotated)
}

async fn check_duplicate(
    ctx: &AppContext,
    row: &domain::models::ImportRow,
) -> Result<bool, DomainError> {
    if let Some(ext_id) = &row.external_metadata_id
        && let Ok(eid) = ExternalMetadataId::new(ext_id.clone())
        && ctx
            .movie_repository
            .get_movie_by_external_id(&eid)
            .await?
            .is_some()
    {
        return Ok(true);
    }
    if let (Some(title), Some(year_str)) = (&row.title, &row.release_year) {
        let title_vo = MovieTitle::new(title.clone());
        let year_vo = year_str
            .parse::<u16>()
            .ok()
            .and_then(|y| ReleaseYear::new(y).ok());
        if let (Ok(t), Some(y)) = (title_vo, year_vo) {
            let matches = ctx
                .movie_repository
                .get_movies_by_title_and_year(&t, &y)
                .await?;
            if !matches.is_empty() {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
