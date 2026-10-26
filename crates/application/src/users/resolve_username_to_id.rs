//! Absorbs `http-axum/src/html/users.rs::get_user_by_username`'s
//! `repos.user.find_by_username()` call. `None` means "no such username" — the
//! handler turns that into a 404, same as it always has; this is not an error
//! condition.

use domain::{
    errors::DomainError,
    value_objects::{UserId, Username},
};

use crate::users::deps::ResolveUsernameDeps;

pub async fn execute(
    deps: &ResolveUsernameDeps,
    username: &Username,
) -> Result<Option<UserId>, DomainError> {
    let user = deps.user.find_by_username(username).await?;
    Ok(user.map(|u| u.id().clone()))
}
