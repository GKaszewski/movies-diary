use domain::errors::DomainError;

mod command;
mod query;
mod rows;

fn infra_err(e: impl std::fmt::Display) -> DomainError {
    DomainError::InfrastructureError(e.to_string())
}
