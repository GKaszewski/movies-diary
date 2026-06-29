use async_trait::async_trait;
use chrono::{NaiveDate, NaiveDateTime};
use uuid::Uuid;

use crate::{
    errors::DomainError,
    models::wrapup::{
        DateRange, WrapUpMovieRow, WrapUpRecord, WrapUpReport, WrapUpScope, WrapUpStatus,
    },
    value_objects::WrapUpId,
};

#[async_trait]
pub trait WrapUpRepository: Send + Sync {
    async fn create(&self, record: &WrapUpRecord) -> Result<(), DomainError>;
    async fn update_status(
        &self,
        id: &WrapUpId,
        status: &WrapUpStatus,
        error: Option<&str>,
    ) -> Result<(), DomainError>;
    async fn set_complete(&self, id: &WrapUpId, report: &WrapUpReport) -> Result<(), DomainError>;
    async fn get_by_id(&self, id: &WrapUpId) -> Result<Option<WrapUpRecord>, DomainError>;
    async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<WrapUpRecord>, DomainError>;
    async fn list_global(&self) -> Result<Vec<WrapUpRecord>, DomainError>;
    async fn find_existing(
        &self,
        user_id: Option<Uuid>,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Option<WrapUpRecord>, DomainError>;
    async fn delete(&self, id: &WrapUpId) -> Result<(), DomainError>;
    async fn delete_failed_older_than(&self, before: NaiveDateTime) -> Result<u64, DomainError>;
}

#[async_trait]
pub trait WrapUpStatsQuery: Send + Sync {
    async fn get_reviews_with_profiles(
        &self,
        scope: &WrapUpScope,
        range: &DateRange,
    ) -> Result<Vec<WrapUpMovieRow>, DomainError>;
}
