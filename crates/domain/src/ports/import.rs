use async_trait::async_trait;

use crate::{
    errors::DomainError,
    models::{
        AnnotatedRow, FieldMapping, FileFormat, ImportError, ImportProfile, ImportSession,
        ParsedFile,
    },
    value_objects::{ImportProfileId, ImportSessionId, UserId},
};

pub trait DocumentParser: Send + Sync {
    fn parse(&self, bytes: &[u8], format: FileFormat) -> Result<ParsedFile, ImportError>;
    fn apply_mapping(&self, file: &ParsedFile, mappings: &[FieldMapping]) -> Vec<AnnotatedRow>;
}

#[async_trait]
pub trait ImportSessionRepository: Send + Sync {
    async fn create(&self, session: &ImportSession) -> Result<(), DomainError>;
    async fn get(
        &self,
        id: &ImportSessionId,
        user_id: &UserId,
    ) -> Result<Option<ImportSession>, DomainError>;
    async fn update(&self, session: &ImportSession) -> Result<(), DomainError>;
    async fn delete(&self, id: &ImportSessionId) -> Result<(), DomainError>;
    async fn delete_expired(&self) -> Result<u64, DomainError>;
    async fn delete_expired_for_user(&self, user_id: &UserId) -> Result<(), DomainError>;
}

#[async_trait]
pub trait ImportProfileRepository: Send + Sync {
    async fn save(&self, profile: &ImportProfile) -> Result<(), DomainError>;
    async fn list_for_user(&self, user_id: &UserId) -> Result<Vec<ImportProfile>, DomainError>;
    async fn get(
        &self,
        id: &ImportProfileId,
        user_id: &UserId,
    ) -> Result<Option<ImportProfile>, DomainError>;
    async fn delete(&self, id: &ImportProfileId) -> Result<(), DomainError>;
}
