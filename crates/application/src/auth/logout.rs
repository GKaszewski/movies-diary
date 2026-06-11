use domain::errors::DomainError;

use crate::context::AppContext;

pub async fn execute(ctx: &AppContext, refresh_token: &str) -> Result<(), DomainError> {
    ctx.repos.refresh_session.revoke(refresh_token).await
}

#[cfg(test)]
#[path = "tests/logout.rs"]
mod tests;
