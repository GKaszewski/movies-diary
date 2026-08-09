use domain::errors::DomainError;
use domain::models::FederatedProfile;
use uuid::Uuid;

use crate::users::deps::GetFederatedProfileDeps;

/// Looks up a federated (remote) profile by synthetic user id — the fallback
/// path `get_user_profile` (`handlers/users.rs`) takes when the local-profile
/// lookup comes back `NotFound`.
///
/// Federation being disabled (`deps.federated_profile` is `None`) collapses to
/// `Ok(None)`, the same value a present-but-empty lookup would return. That's
/// deliberate: the handler's `if let Ok(Some(fed)) = ...` treats "port absent",
/// "call returned `Ok(None)`", and "call returned `Err`" identically — all three
/// fall through to a 404 — and this use case must not make any of those three
/// distinguishable to a caller that cannot act on the difference. An `Err` from
/// the port is propagated unchanged; the swallow stays at the handler, exactly
/// where it already was.
pub async fn execute(
    deps: &GetFederatedProfileDeps,
    user_id: Uuid,
) -> Result<Option<FederatedProfile>, DomainError> {
    match &deps.federated_profile {
        Some(fed_query) => fed_query.get_federated_profile(user_id).await,
        None => Ok(None),
    }
}

#[cfg(test)]
#[path = "tests/get_federated_profile.rs"]
mod tests;
