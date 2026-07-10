use std::sync::Arc;

use domain::{errors::DomainError, ports::UserRepository};

use crate::users::queries::GetCurrentProfileQuery;

pub struct ProfileFieldData {
    pub name: String,
    pub value: String,
}

pub struct CurrentProfileData {
    pub username: String,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_path: Option<String>,
    pub banner_path: Option<String>,
    pub also_known_as: Option<String>,
    pub fields: Vec<ProfileFieldData>,
    pub role: domain::models::UserRole,
}

pub async fn execute(
    user: Arc<dyn UserRepository>,
    query: GetCurrentProfileQuery,
) -> Result<CurrentProfileData, DomainError> {
    let user_id = domain::value_objects::UserId::from_uuid(query.user_id);
    let found = user
        .find_by_id(&user_id)
        .await?
        .ok_or_else(|| DomainError::NotFound("User not found".into()))?;

    let fields = found
        .profile_fields()
        .iter()
        .map(|f| ProfileFieldData {
            name: f.name.clone(),
            value: f.value.clone(),
        })
        .collect();

    Ok(CurrentProfileData {
        username: found.username().value().to_string(),
        display_name: found.display_name().map(|s| s.to_string()),
        bio: found.bio().map(|s| s.to_string()),
        avatar_path: found.avatar_path().map(|s| s.to_string()),
        banner_path: found.banner_path().map(|s| s.to_string()),
        also_known_as: found.also_known_as().map(|s| s.to_string()),
        fields,
        role: found.role().clone(),
    })
}

#[cfg(test)]
#[path = "tests/get_current_profile.rs"]
mod tests;
