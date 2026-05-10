use domain::errors::DomainError;
use crate::context::AppContext;

pub async fn execute(ctx: &AppContext) -> Result<u64, DomainError> {
    ctx.import_session_repository.delete_expired().await
}
