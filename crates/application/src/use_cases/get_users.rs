use crate::{context::AppContext, queries::GetUsersQuery};
use domain::{errors::DomainError, models::UserSummary, ports::RemoteActorInfo};

pub struct UsersListData {
    pub users: Vec<UserSummary>,
    pub remote_actors: Vec<RemoteActorInfo>,
}

pub async fn execute(
    ctx: &AppContext,
    _query: GetUsersQuery,
) -> Result<UsersListData, DomainError> {
    #[cfg(feature = "federation")]
    let (users_result, actors_result) = tokio::join!(
        ctx.user_repository.list_with_stats(),
        ctx.social_query.list_all_followed_remote_actors()
    );
    #[cfg(not(feature = "federation"))]
    let (users_result, actors_result) = (
        ctx.user_repository.list_with_stats().await,
        Ok::<Vec<RemoteActorInfo>, DomainError>(vec![]),
    );

    Ok(UsersListData {
        users: users_result?,
        remote_actors: actors_result?,
    })
}
