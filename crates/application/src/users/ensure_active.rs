use domain::{errors::DomainError, value_objects::UserId};

use crate::users::deps::EnsureActiveDeps;

pub async fn execute(deps: &EnsureActiveDeps, user: &UserId) -> Result<(), DomainError> {
    match deps.user.find_by_id(user).await? {
        Some(_) => Ok(()),
        None => Err(DomainError::Unauthorized("account is not active".into())),
    }
}
