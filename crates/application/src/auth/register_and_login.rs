use domain::errors::DomainError;

use crate::{
    auth::commands::RegisterAndLoginCommand,
    auth::{login, register},
    context::AppContext,
};

pub async fn execute(
    ctx: &AppContext,
    cmd: RegisterAndLoginCommand,
) -> Result<login::LoginResult, DomainError> {
    register::execute(
        ctx,
        crate::auth::commands::RegisterCommand {
            email: cmd.email.clone(),
            username: cmd.username,
            password: cmd.password.clone(),
            role: domain::models::UserRole::Standard,
        },
    )
    .await?;

    login::execute(
        ctx,
        crate::auth::queries::LoginQuery {
            email: cmd.email,
            password: cmd.password,
        },
    )
    .await
}
