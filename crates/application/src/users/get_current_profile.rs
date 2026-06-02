use domain::errors::DomainError;

use crate::{context::AppContext, users::queries::GetCurrentProfileQuery};

pub struct CurrentProfileData {
    pub username: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
}

pub async fn execute(
    ctx: &AppContext,
    query: GetCurrentProfileQuery,
) -> Result<CurrentProfileData, DomainError> {
    let user_id = domain::value_objects::UserId::from_uuid(query.user_id);
    let user = ctx
        .repos
        .user
        .find_by_id(&user_id)
        .await?
        .ok_or_else(|| DomainError::NotFound("User not found".into()))?;

    let avatar_url = user
        .avatar_path()
        .map(|path| format!("{}/images/{}", ctx.config.base_url, path));

    Ok(CurrentProfileData {
        username: user.username().value().to_string(),
        bio: user.bio().map(|s| s.to_string()),
        avatar_url,
    })
}
