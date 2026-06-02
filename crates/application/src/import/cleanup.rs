use crate::context::AppContext;
use domain::errors::DomainError;

pub async fn execute(ctx: &AppContext) -> Result<u64, DomainError> {
    ctx.repos.import_session.delete_expired().await
}
