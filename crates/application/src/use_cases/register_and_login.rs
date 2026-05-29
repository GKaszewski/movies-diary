use domain::errors::DomainError;

use crate::{
    commands::RegisterAndLoginCommand,
    context::AppContext,
    use_cases::{login, register},
};

pub async fn execute(
    ctx: &AppContext,
    cmd: RegisterAndLoginCommand,
) -> Result<login::LoginResult, DomainError> {
    register::execute(
        ctx,
        crate::commands::RegisterCommand {
            email: cmd.email.clone(),
            username: cmd.username,
            password: cmd.password.clone(),
            role: domain::models::UserRole::Standard,
        },
    )
    .await?;

    login::execute(
        ctx,
        crate::queries::LoginQuery {
            email: cmd.email,
            password: cmd.password,
        },
    )
    .await
}
