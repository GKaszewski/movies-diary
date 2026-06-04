use domain::errors::DomainError;

use crate::{context::AppContext, users::queries::GetCurrentProfileQuery};

pub struct ProfileFieldData {
    pub name: String,
    pub value: String,
}

pub struct CurrentProfileData {
    pub username: String,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub banner_url: Option<String>,
    pub also_known_as: Option<String>,
    pub fields: Vec<ProfileFieldData>,
    pub role: String,
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
    let banner_url = user
        .banner_path()
        .map(|path| format!("{}/images/{}", ctx.config.base_url, path));

    let fields = user
        .profile_fields()
        .iter()
        .map(|f| ProfileFieldData {
            name: f.name.clone(),
            value: f.value.clone(),
        })
        .collect();

    Ok(CurrentProfileData {
        username: user.username().value().to_string(),
        display_name: user.display_name().map(|s| s.to_string()),
        bio: user.bio().map(|s| s.to_string()),
        avatar_url,
        banner_url,
        also_known_as: user.also_known_as().map(|s| s.to_string()),
        fields,
        role: user.role().as_str().into(),
    })
}
