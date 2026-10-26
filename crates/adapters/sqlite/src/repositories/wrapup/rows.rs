use chrono::NaiveDate;
use domain::{
    errors::DomainError,
    models::wrapup::{WrapUpRecord, WrapUpStatus},
    value_objects::WrapUpId,
};
use sqlx::Row;

use crate::shared::{parse_datetime, parse_uuid};

pub(super) fn status_to_str(s: &WrapUpStatus) -> &'static str {
    match s {
        WrapUpStatus::Pending => "pending",
        WrapUpStatus::Generating => "generating",
        WrapUpStatus::Ready => "ready",
        WrapUpStatus::Failed => "failed",
    }
}

pub(super) fn parse_status(s: &str) -> Result<WrapUpStatus, DomainError> {
    match s {
        "pending" => Ok(WrapUpStatus::Pending),
        "generating" => Ok(WrapUpStatus::Generating),
        "ready" => Ok(WrapUpStatus::Ready),
        "failed" => Ok(WrapUpStatus::Failed),
        other => Err(DomainError::InfrastructureError(format!(
            "Unknown wrap-up status: {other}"
        ))),
    }
}

pub(super) fn parse_date(s: &str) -> Result<NaiveDate, DomainError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| DomainError::InfrastructureError(format!("Invalid date '{s}': {e}")))
}

pub(super) fn row_to_record(row: &sqlx::sqlite::SqliteRow) -> Result<WrapUpRecord, DomainError> {
    let id_str: String = row.try_get("id").map_err(crate::shared::map_sqlx_error)?;
    let user_id_str: Option<String> = row
        .try_get("user_id")
        .map_err(crate::shared::map_sqlx_error)?;
    let start_date_str: String = row
        .try_get("start_date")
        .map_err(crate::shared::map_sqlx_error)?;
    let end_date_str: String = row
        .try_get("end_date")
        .map_err(crate::shared::map_sqlx_error)?;
    let status_str: String = row
        .try_get("status")
        .map_err(crate::shared::map_sqlx_error)?;
    let report_json: Option<String> = row
        .try_get("report_json")
        .map_err(crate::shared::map_sqlx_error)?;
    let error_message: Option<String> = row
        .try_get("error_message")
        .map_err(crate::shared::map_sqlx_error)?;
    let created_at_str: String = row
        .try_get("created_at")
        .map_err(crate::shared::map_sqlx_error)?;
    let completed_at_str: Option<String> = row
        .try_get("completed_at")
        .map_err(crate::shared::map_sqlx_error)?;

    let user_id = user_id_str.as_deref().map(parse_uuid).transpose()?;

    let report = report_json.and_then(|j| serde_json::from_str(&j).ok());

    Ok(WrapUpRecord {
        id: WrapUpId::from_uuid(parse_uuid(&id_str)?),
        user_id,
        start_date: parse_date(&start_date_str)?,
        end_date: parse_date(&end_date_str)?,
        status: parse_status(&status_str)?,
        report,
        error_message,
        created_at: parse_datetime(&created_at_str)?,
        completed_at: completed_at_str
            .as_deref()
            .map(parse_datetime)
            .transpose()?,
    })
}
