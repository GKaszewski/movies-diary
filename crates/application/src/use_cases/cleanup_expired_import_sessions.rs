use crate::context::AppContext;
use domain::errors::DomainError;

pub async fn execute(ctx: &AppContext) -> Result<u64, DomainError> {
    ctx.import_session_repository.delete_expired().await
}
