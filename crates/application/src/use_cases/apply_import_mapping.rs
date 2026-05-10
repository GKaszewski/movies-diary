use domain::{
    errors::DomainError,
    value_objects::{ExternalMetadataId, ImportSessionId, MovieTitle, ReleaseYear, UserId},
};
use importer::{AnnotatedRow, ParsedFile, apply_mapping};

use crate::{commands::ApplyImportMappingCommand, context::AppContext};

pub async fn execute(ctx: &AppContext, cmd: ApplyImportMappingCommand) -> Result<Vec<AnnotatedRow>, DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    let session_id = ImportSessionId::from_uuid(cmd.session_id);
    let mappings = cmd.mappings;
    let mut session = ctx.import_session_repository
        .get(&session_id, &user_id)
        .await?
        .ok_or_else(|| DomainError::NotFound("import session".into()))?;

    let parsed: ParsedFile = serde_json::from_str(&session.parsed_data)
        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

    let mut annotated = apply_mapping(&parsed, &mappings);

    for row in annotated.iter_mut() {
        if let importer::RowResult::Valid(ref import_row) = row.result {
            row.is_duplicate = check_duplicate(ctx, import_row).await?;
        }
    }

    session.field_mappings = Some(
        serde_json::to_string(&mappings)
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?
    );
    session.row_results = Some(
        serde_json::to_string(&annotated)
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?
    );

    ctx.import_session_repository.update(&session).await?;

    Ok(annotated)
}

async fn check_duplicate(ctx: &AppContext, row: &importer::ImportRow) -> Result<bool, DomainError> {
    if let Some(ext_id) = &row.external_metadata_id {
        if let Ok(eid) = ExternalMetadataId::new(ext_id.clone()) {
            if ctx.movie_repository.get_movie_by_external_id(&eid).await?.is_some() {
                return Ok(true);
            }
        }
    }
    if let (Some(title), Some(year_str)) = (&row.title, &row.release_year) {
        let title_vo = MovieTitle::new(title.clone());
        let year_vo = year_str.parse::<u16>().ok().and_then(|y| ReleaseYear::new(y).ok());
        if let (Ok(t), Some(y)) = (title_vo, year_vo) {
            let matches = ctx.movie_repository.get_movies_by_title_and_year(&t, &y).await?;
            if !matches.is_empty() {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
