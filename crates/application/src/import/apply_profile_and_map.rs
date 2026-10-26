//! Absorbs `http-axum/src/api/import/sessions.rs::api_apply_profile`'s three-step orchestration:
//! apply the saved profile's field mappings onto the session, reload the
//! session to read back the mappings `apply_profile` just wrote, then run
//! `apply_mapping` to regenerate `row_results` from them. All three steps used
//! to live in the handler; this use case is the only caller-visible change —
//! the two existing use cases it drives (`apply_profile::execute`,
//! `apply_mapping::execute`) are untouched, per this task's constraint against
//! reshaping already-existing use-case signatures.

use domain::{errors::DomainError, value_objects::ImportSessionId};

use crate::import::{
    apply_mapping, apply_profile,
    commands::{ApplyImportMappingCommand, ApplyImportProfileCommand, ApplyProfileAndMapCommand},
    deps::{ApplyMappingDeps, ApplyProfileAndMapDeps, ApplyProfileDeps},
};

pub async fn execute(
    deps: &ApplyProfileAndMapDeps,
    cmd: ApplyProfileAndMapCommand,
) -> Result<Vec<domain::models::AnnotatedRow>, DomainError> {
    let profile_deps = ApplyProfileDeps {
        import_profile: deps.import_profile.clone(),
        import_session: deps.import_session.clone(),
    };
    apply_profile::execute(
        &profile_deps,
        ApplyImportProfileCommand {
            user_id: cmd.user_id,
            session_id: cmd.session_id,
            profile_id: cmd.profile_id,
        },
    )
    .await?;

    let session_id = ImportSessionId::from_uuid(cmd.session_id);
    let user_id = domain::value_objects::UserId::from_uuid(cmd.user_id);
    let session = deps
        .import_session
        .get(&session_id, &user_id)
        .await?
        .ok_or_else(|| DomainError::NotFound("session not found after profile apply".into()))?;

    let mappings = session.field_mappings.unwrap_or_default();

    let mapping_deps = ApplyMappingDeps {
        import_session: deps.import_session.clone(),
        document_parser: deps.document_parser.clone(),
        movie_query: deps.movie_query.clone(),
    };
    apply_mapping::execute(
        &mapping_deps,
        ApplyImportMappingCommand {
            user_id: cmd.user_id,
            session_id: cmd.session_id,
            mappings,
        },
    )
    .await
}
