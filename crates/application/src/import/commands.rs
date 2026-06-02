use domain::models::{FieldMapping, FileFormat};
use uuid::Uuid;

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
