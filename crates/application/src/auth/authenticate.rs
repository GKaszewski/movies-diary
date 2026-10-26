use domain::{errors::DomainError, value_objects::UserId};

use crate::auth::deps::AuthenticateDeps;

pub async fn execute(deps: &AuthenticateDeps, token: &str) -> Result<UserId, DomainError> {
    let user_id = deps.auth.validate_token(token).await?;
    match deps.user.find_by_id(&user_id).await? {
        Some(_) => Ok(user_id),
        None => Err(DomainError::Unauthorized("account is not active".into())),
    }
}
