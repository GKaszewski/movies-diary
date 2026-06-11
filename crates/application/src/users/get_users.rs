use std::sync::Arc;

use crate::users::queries::GetUsersQuery;
use domain::{errors::DomainError, models::UserSummary, ports::{RemoteActorInfo, SocialQueryPort, UserRepository}};

pub struct UsersListData {
    pub users: Vec<UserSummary>,
    pub remote_actors: Vec<RemoteActorInfo>,
}

pub async fn execute(
    user: Arc<dyn UserRepository>,
    social_query: Arc<dyn SocialQueryPort>,
    _query: GetUsersQuery,
) -> Result<UsersListData, DomainError> {
    let (users_result, actors_result) = tokio::join!(
        user.list_with_stats(),
        social_query.list_all_followed_remote_actors()
    );

    Ok(UsersListData {
        users: users_result?,
        remote_actors: actors_result?,
    })
}

#[cfg(test)]
#[path = "tests/get_users.rs"]
mod tests;
