use chrono::NaiveDate;
use domain::{
    errors::DomainError,
    models::wrapup::{WrapUpReport, WrapUpScope, WrapUpStatus},
};

use crate::wrapup::deps::GetReadyReportDeps;

/// Absorbs the year -> date-range conversion and the `status == Ready` +
/// `report.is_some()` gating both HTML wrap-up handlers used to do by hand.
///
/// Takes `domain::models::wrapup::WrapUpScope` directly (the same type
/// `wrapup::handle_requested` already matches on) rather than a second,
/// use-case-local `WrapUpScope` — there is exactly one concept here, one type.
/// `None` can no longer be silently passed for "global": callers must pick
/// `WrapUpScope::User(uid)` or `WrapUpScope::Global` explicitly.
///
/// Error shape mirrors the deleted handlers exactly:
/// - An unrepresentable `year` (can't form a valid `NaiveDate`) is
///   `ValidationError` — the handlers used this to return a bare 400.
/// - Everything else — no record, a record whose `status` isn't `Ready`, a
///   `Ready` record with `report: None`, or even a repository error — collapses
///   to `NotFound`, matching the old handlers' `_ => return StatusCode::NOT_FOUND`
///   catch-all (which really did swallow repo errors into a 404, not just the
///   "not ready yet" cases; preserved here rather than tidied).
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
