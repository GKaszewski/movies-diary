use std::sync::Arc;

use domain::ports::{AuthService, PasswordHasher, RefreshSessionRepository, UserRepository};

use crate::config::AppSettings;

pub struct LoginDeps {
    pub user: Arc<dyn UserRepository>,
    pub password_hasher: Arc<dyn PasswordHasher>,
    pub auth: Arc<dyn AuthService>,
    pub refresh_session: Arc<dyn RefreshSessionRepository>,
    pub config: AppSettings,
}

pub struct RegisterDeps {
    pub user: Arc<dyn UserRepository>,
    pub password_hasher: Arc<dyn PasswordHasher>,
    pub config: AppSettings,
}

pub struct RefreshDeps {
    pub refresh_session: Arc<dyn RefreshSessionRepository>,
    pub auth: Arc<dyn AuthService>,
    pub config: AppSettings,
}

pub struct RegisterAndLoginDeps {
    pub user: Arc<dyn UserRepository>,
    pub password_hasher: Arc<dyn PasswordHasher>,
    pub auth: Arc<dyn AuthService>,
    pub refresh_session: Arc<dyn RefreshSessionRepository>,
    pub config: AppSettings,
}

pub struct AuthenticateDeps {
    pub auth: Arc<dyn AuthService>,
    pub user: Arc<dyn UserRepository>,
}

pub struct LogoutDeps {
    pub refresh_session: Arc<dyn RefreshSessionRepository>,
}
