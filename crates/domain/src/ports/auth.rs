use async_trait::async_trait;

use crate::{
    errors::DomainError,
    models::{GeneratedToken, RefreshSession, User, UserSettings, UserSummary},
    value_objects::{Email, PasswordHash, UserId, Username},
};

#[async_trait]
pub trait AuthService: Send + Sync {
    async fn generate_token(&self, user_id: &UserId) -> Result<GeneratedToken, DomainError>;
    async fn validate_token(&self, token: &str) -> Result<UserId, DomainError>;
}

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, DomainError>;
    async fn find_by_username(&self, username: &Username) -> Result<Option<User>, DomainError>;
    async fn save(&self, user: &User) -> Result<(), DomainError>;
    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, DomainError>;
    async fn list_with_stats(&self) -> Result<Vec<UserSummary>, DomainError>;
    async fn update_profile(
        &self,
        user_id: &UserId,
        profile: &crate::models::UserProfile,
    ) -> Result<(), DomainError>;
}

#[async_trait]
pub trait UserProfileFieldsRepository: Send + Sync {
    async fn get_fields(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<crate::models::ProfileField>, DomainError>;
    async fn set_fields(
        &self,
        user_id: &UserId,
        fields: Vec<crate::models::ProfileField>,
    ) -> Result<(), DomainError>;
}

#[async_trait]
pub trait PasswordHasher: Send + Sync {
    async fn hash(&self, plain_password: &str) -> Result<PasswordHash, DomainError>;
    async fn verify(&self, plain_password: &str, hash: &PasswordHash) -> Result<bool, DomainError>;
}

#[async_trait]
pub trait UserSettingsRepository: Send + Sync {
    async fn get(&self, user_id: &UserId) -> Result<UserSettings, DomainError>;
    async fn save(&self, settings: &UserSettings) -> Result<(), DomainError>;
}

#[async_trait]
pub trait RefreshSessionRepository: Send + Sync {
    async fn create(&self, session: &RefreshSession) -> Result<(), DomainError>;
    async fn get_by_token(&self, token: &str) -> Result<Option<RefreshSession>, DomainError>;
    async fn revoke(&self, token: &str) -> Result<(), DomainError>;
    async fn revoke_all_for_user(&self, user_id: &UserId) -> Result<(), DomainError>;
    async fn delete_expired(&self) -> Result<u64, DomainError>;
}
