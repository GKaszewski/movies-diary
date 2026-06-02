use crate::{context::AppContext, users::queries::GetUsersQuery};
use domain::{errors::DomainError, models::UserSummary, ports::RemoteActorInfo};

pub struct UsersListData {
    pub users: Vec<UserSummary>,
    pub remote_actors: Vec<RemoteActorInfo>,
}

pub async fn execute(
    ctx: &AppContext,
    _query: GetUsersQuery,
) -> Result<UsersListData, DomainError> {
    let (users_result, actors_result) = tokio::join!(
        ctx.repos.user.list_with_stats(),
        ctx.repos.social_query.list_all_followed_remote_actors()
    );

    Ok(UsersListData {
        users: users_result?,
        remote_actors: actors_result?,
    })
}
