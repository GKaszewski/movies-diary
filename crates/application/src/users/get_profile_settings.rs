use domain::{errors::DomainError, value_objects::UserId};
use uuid::Uuid;

use crate::users::deps::GetProfileSettingsDeps;

pub struct ProfileSettings {
    pub username: String,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub banner_url: Option<String>,
    pub also_known_as: Option<String>,
    pub fields: Vec<(String, String)>,
}

pub async fn execute(
    deps: &GetProfileSettingsDeps,
    user_id: Uuid,
) -> Result<ProfileSettings, DomainError> {
    let uid = UserId::from_uuid(user_id);
    let user = deps
        .user
        .find_by_id(&uid)
        .await?
        .ok_or_else(|| DomainError::NotFound(format!("user {}", user_id)))?;

    let avatar_url = user.avatar_path().map(|p| deps.instance.image_url_for(p));
    let banner_url = user.banner_path().map(|p| deps.instance.image_url_for(p));

    let fields = deps
        .profile_fields
        .get_fields(&uid)
        .await?
        .into_iter()
        .map(|f| (f.name, f.value))
        .collect();

    Ok(ProfileSettings {
        username: user.username().value().to_string(),
        display_name: user.display_name().map(str::to_string),
        bio: user.bio().map(str::to_string),
        avatar_url,
        banner_url,
        also_known_as: user.also_known_as().map(str::to_string),
        fields,
    })
}
