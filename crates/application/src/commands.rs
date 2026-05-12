use chrono::NaiveDateTime;
use domain::models::{FieldMapping, FileFormat, UserRole};
use uuid::Uuid;

pub struct LogReviewCommand {
    pub external_metadata_id: Option<String>,

    pub manual_title: Option<String>,
    pub manual_release_year: Option<u16>,
    pub manual_director: Option<String>,

    pub user_id: Uuid,
    pub rating: u8,
    pub comment: Option<String>,
    pub watched_at: NaiveDateTime,
}

#[derive(Clone)]
pub struct SyncPosterCommand {
    pub movie_id: Uuid,
    pub external_metadata_id: String,
}

pub struct RegisterCommand {
    pub email: String,
    pub username: String,
    pub password: String,
    pub role: UserRole,
}

pub struct DeleteReviewCommand {
    pub review_id: Uuid,
    pub requesting_user_id: Uuid,
}

// FileFormat is now in domain::models — no longer defined here

pub struct CreateImportSessionCommand {
    pub user_id: Uuid,
    pub bytes: Vec<u8>,
    pub format: FileFormat,
}

pub struct ApplyImportMappingCommand {
    pub user_id: Uuid,
    pub session_id: Uuid,
    pub mappings: Vec<FieldMapping>,
}

pub struct ExecuteImportCommand {
    pub user_id: Uuid,
    pub session_id: Uuid,
    pub confirmed_indices: Vec<usize>,
}

pub struct SaveImportProfileCommand {
    pub user_id: Uuid,
    pub session_id: Uuid,
    pub name: String,
}

pub struct ApplyImportProfileCommand {
    pub user_id: Uuid,
    pub session_id: Uuid,
    pub profile_id: Uuid,
}

pub struct DeleteImportProfileCommand {
    pub user_id: Uuid,
    pub profile_id: Uuid,
}

pub struct UpdateProfileCommand {
    pub user_id: Uuid,
    pub bio: Option<String>,
    pub avatar_bytes: Option<Vec<u8>>,
    pub avatar_content_type: Option<String>,
}
