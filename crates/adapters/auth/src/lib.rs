use argon2::{
    Argon2,
    password_hash::{PasswordHasher as _, PasswordVerifier, SaltString},
};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use domain::{
    errors::DomainError,
    ports::{AuthService, GeneratedToken, PasswordHasher},
    value_objects::{PasswordHash, UserId},
};

pub struct AuthConfig {
    secret: String,
    ttl_seconds: u64,
}

impl AuthConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let secret = std::env::var("JWT_SECRET")
            .map_err(|_| anyhow::anyhow!("JWT_SECRET env var is required"))?;
        if secret.is_empty() {
            anyhow::bail!("JWT_SECRET must not be empty");
        }
        let ttl_seconds = std::env::var("JWT_TTL_SECONDS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(86400u64);
        Ok(Self {
            secret,
            ttl_seconds,
        })
    }
}

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: u64,
}

pub struct JwtAuthService {
    config: AuthConfig,
}

impl JwtAuthService {
    pub fn new(config: AuthConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl AuthService for JwtAuthService {
    async fn generate_token(&self, user_id: &UserId) -> Result<GeneratedToken, DomainError> {
        let expires_at = Utc::now() + Duration::seconds(self.config.ttl_seconds as i64);
        let claims = Claims {
            sub: user_id.value().to_string(),
            exp: expires_at.timestamp() as u64,
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.config.secret.as_bytes()),
        )
        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        Ok(GeneratedToken { token, expires_at })
    }

    async fn validate_token(&self, token: &str) -> Result<UserId, DomainError> {
        let data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.config.secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| DomainError::Unauthorized("Invalid or expired token".into()))?;
        let uuid = Uuid::parse_str(&data.claims.sub)
            .map_err(|_| DomainError::Unauthorized("Invalid token subject".into()))?;
        Ok(UserId::from_uuid(uuid))
    }
}

pub struct Argon2PasswordHasher;

#[async_trait]
impl PasswordHasher for Argon2PasswordHasher {
    async fn hash(&self, plain_password: &str) -> Result<PasswordHash, DomainError> {
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(plain_password.as_bytes(), &salt)
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?
            .to_string();
        PasswordHash::new(hash).map_err(|e| DomainError::InfrastructureError(e.to_string()))
    }

    async fn verify(&self, plain_password: &str, hash: &PasswordHash) -> Result<bool, DomainError> {
        let parsed = argon2::password_hash::PasswordHash::new(hash.value())
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        Ok(Argon2::default()
            .verify_password(plain_password.as_bytes(), &parsed)
            .is_ok())
    }
}

pub fn create() -> anyhow::Result<(
    std::sync::Arc<dyn domain::ports::AuthService>,
    std::sync::Arc<dyn domain::ports::PasswordHasher>,
)> {
    let config = AuthConfig::from_env()?;
    Ok((
        std::sync::Arc::new(JwtAuthService::new(config)),
        std::sync::Arc::new(Argon2PasswordHasher),
    ))
}
