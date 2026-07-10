use std::sync::Arc;

use domain::{
    errors::DomainError,
    models::{AnnotatedRow, import::RowResult},
    ports::MovieQuery,
    value_objects::{ExternalMetadataId, ImportSessionId, MovieTitle, ReleaseYear, UserId},
};

use super::{commands::ApplyImportMappingCommand, deps::ApplyMappingDeps};

pub async fn execute(
    deps: &ApplyMappingDeps,
    cmd: ApplyImportMappingCommand,
) -> Result<Vec<AnnotatedRow>, DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    let session_id = ImportSessionId::from_uuid(cmd.session_id);
    let mappings = cmd.mappings;
    let mut session = deps
        .import_session
        .get(&session_id, &user_id)
        .await?
        .ok_or_else(|| DomainError::NotFound("import session".into()))?;

    let parsed = session
        .parsed_file
        .clone()
        .ok_or_else(|| DomainError::ValidationError("session has no parsed file".into()))?;

    let mut annotated = deps.document_parser.apply_mapping(&parsed, &mappings);

    mark_duplicates(deps.movie_query.clone(), &mut annotated).await?;

    session.field_mappings = Some(mappings);
    session.row_results = Some(annotated.clone());

    deps.import_session.update(&session).await?;

    Ok(annotated)
}

async fn mark_duplicates(
    movie: Arc<dyn MovieQuery>,
    rows: &mut [AnnotatedRow],
) -> Result<(), DomainError> {
    let mut ext_ids = Vec::new();
    let mut title_year_pairs = Vec::new();

    for row in rows.iter() {
        if let RowResult::Valid(ref r) = row.result {
            if let Some(ext_id) = &r.external_metadata_id
                && let Ok(eid) = ExternalMetadataId::new(ext_id.clone())
            {
                ext_ids.push(eid);
            }
            if let (Some(title), Some(year_str)) = (&r.title, &r.release_year)
                && let Ok(t) = MovieTitle::new(title.clone())
                && let Some(y) = year_str
                    .parse::<u16>()
                    .ok()
                    .and_then(|y| ReleaseYear::new(y).ok())
            {
                title_year_pairs.push((t, y));
            }
        }
    }

    let known_ext = movie.existing_external_ids(&ext_ids).await?;
    let known_ty = movie.existing_title_year_pairs(&title_year_pairs).await?;

    for row in rows.iter_mut() {
        if let RowResult::Valid(ref r) = row.result {
            if let Some(ext_id) = &r.external_metadata_id
                && known_ext.contains(ext_id)
            {
                row.is_duplicate = true;
                continue;
            }
            if let (Some(title), Some(year_str)) = (&r.title, &r.release_year)
                && let Ok(y) = year_str.parse::<u16>()
                && known_ty.contains(&(title.clone(), y))
            {
                row.is_duplicate = true;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "tests/apply_mapping.rs"]
mod tests;
