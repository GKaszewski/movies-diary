use chrono::NaiveDate;
use domain::{
    errors::DomainError,
    models::wrapup::{WrapUpReport, WrapUpScope, WrapUpStatus},
};

use crate::wrapup::deps::GetReadyReportDeps;

pub async fn execute(
    deps: &GetReadyReportDeps,
    scope: WrapUpScope,
    year: i32,
) -> Result<WrapUpReport, DomainError> {
    let start = NaiveDate::from_ymd_opt(year, 1, 1)
        .ok_or_else(|| DomainError::ValidationError("invalid year".into()))?;
    let end = NaiveDate::from_ymd_opt(year + 1, 1, 1)
        .ok_or_else(|| DomainError::ValidationError("invalid year".into()))?;

    let user_id = match scope {
        WrapUpScope::User(uid) => Some(uid),
        WrapUpScope::Global => None,
    };

    let record = match deps.wrapup_repo.find_existing(user_id, start, end).await {
        Ok(Some(r)) if r.status == WrapUpStatus::Ready => r,
        _ => return Err(DomainError::NotFound("wrap-up report".into())),
    };

    record
        .report
        .ok_or_else(|| DomainError::NotFound("wrap-up report".into()))
}

#[cfg(test)]
#[path = "tests/get_ready_report.rs"]
mod tests;
