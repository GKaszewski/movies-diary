use uuid::Uuid;

pub struct UpdateProfileCommand {
    pub user_id: Uuid,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_bytes: Option<Vec<u8>>,
    pub avatar_content_type: Option<String>,
    pub banner_bytes: Option<Vec<u8>>,
    pub banner_content_type: Option<String>,
    pub also_known_as: Option<String>,
}

pub struct UpdateProfileFieldsCommand {
    pub user_id: Uuid,
    pub fields: Vec<domain::models::ProfileField>,
}
