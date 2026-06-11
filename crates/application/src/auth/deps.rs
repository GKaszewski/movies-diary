use std::sync::Arc;

use domain::ports::{AuthService, PasswordHasher, RefreshSessionRepository, UserRepository};

use crate::config::AppConfig;

pub struct LoginDeps {
    pub user: Arc<dyn UserRepository>,
    pub password_hasher: Arc<dyn PasswordHasher>,
    pub auth: Arc<dyn AuthService>,
    pub refresh_session: Arc<dyn RefreshSessionRepository>,
    pub config: AppConfig,
}

pub struct RegisterDeps {
    pub user: Arc<dyn UserRepository>,
    pub password_hasher: Arc<dyn PasswordHasher>,
    pub config: AppConfig,
}

pub struct RefreshDeps {
    pub refresh_session: Arc<dyn RefreshSessionRepository>,
    pub auth: Arc<dyn AuthService>,
    pub config: AppConfig,
}

pub struct RegisterAndLoginDeps {
    pub user: Arc<dyn UserRepository>,
    pub password_hasher: Arc<dyn PasswordHasher>,
    pub auth: Arc<dyn AuthService>,
    pub refresh_session: Arc<dyn RefreshSessionRepository>,
    pub config: AppConfig,
}
