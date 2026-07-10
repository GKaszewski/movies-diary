use crate::users::{deps::GetUsersListDeps, queries::GetUsersQuery};
use domain::{
    errors::DomainError,
    models::{RemoteActorInfo, UserSummary},
};

pub struct UsersListData {
    pub users: Vec<UserSummary>,
    pub remote_actors: Vec<RemoteActorInfo>,
}

pub async fn execute(
    deps: &GetUsersListDeps,
    _query: GetUsersQuery,
) -> Result<UsersListData, DomainError> {
    let (users_result, actors_result) = tokio::join!(
        deps.user.list_with_stats(),
        deps.social_query_legacy.list_all_followed_remote_actors()
    );

    Ok(UsersListData {
        users: users_result?,
        remote_actors: actors_result?,
    })
}
