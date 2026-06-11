use domain::errors::DomainError;

use crate::auth::{
    commands::{RegisterAndLoginCommand, RegisterCommand},
    deps::{LoginDeps, RegisterAndLoginDeps, RegisterDeps},
    login::{self, LoginResult},
    queries::LoginQuery,
    register,
};

pub async fn execute(
    deps: &RegisterAndLoginDeps,
    cmd: RegisterAndLoginCommand,
) -> Result<LoginResult, DomainError> {
    let reg_deps = RegisterDeps {
        user: deps.user.clone(),
        password_hasher: deps.password_hasher.clone(),
        config: deps.config.clone(),
    };
    register::execute(
        &reg_deps,
        RegisterCommand {
            email: cmd.email.clone(),
            username: cmd.username,
            password: cmd.password.clone(),
            role: domain::models::UserRole::Standard,
        },
    )
    .await?;

    let log_deps = LoginDeps {
        user: deps.user.clone(),
        password_hasher: deps.password_hasher.clone(),
        auth: deps.auth.clone(),
        refresh_session: deps.refresh_session.clone(),
        config: deps.config.clone(),
    };
    login::execute(
        &log_deps,
        LoginQuery {
            email: cmd.email,
            password: cmd.password,
        },
    )
    .await
}

#[cfg(test)]
#[path = "tests/register_and_login.rs"]
mod tests;
