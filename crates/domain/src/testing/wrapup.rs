use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use uuid::Uuid;

use crate::{errors::DomainError, ports::WrapUpRepository, value_objects::WrapUpId};

// ── PanicWrapUpStatsQuery ───────────────────────────────────────────────────

pub struct PanicWrapUpStatsQuery;

#[async_trait]
impl crate::ports::WrapUpStatsQuery for PanicWrapUpStatsQuery {
    async fn get_reviews_with_profiles(
        &self,
        _: &crate::models::wrapup::WrapUpScope,
        _: &crate::models::wrapup::DateRange,
    ) -> Result<Vec<crate::models::WrapUpMovieRow>, DomainError> {
        unimplemented!("WrapUpStatsQuery not wired")
    }
}

// ── InMemoryWrapUpStatsQuery ────────────────────────────────────────────────

pub struct InMemoryWrapUpStatsQuery {
    pub rows: Mutex<Vec<crate::models::WrapUpMovieRow>>,
}

impl InMemoryWrapUpStatsQuery {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            rows: Mutex::new(Vec::new()),
        })
    }

    pub fn with_rows(rows: Vec<crate::models::WrapUpMovieRow>) -> Arc<Self> {
        Arc::new(Self {
            rows: Mutex::new(rows),
        })
    }
}

#[async_trait]
impl crate::ports::WrapUpStatsQuery for InMemoryWrapUpStatsQuery {
    async fn get_reviews_with_profiles(
        &self,
        scope: &crate::models::wrapup::WrapUpScope,
        range: &crate::models::wrapup::DateRange,
    ) -> Result<Vec<crate::models::WrapUpMovieRow>, DomainError> {
        let rows = self.rows.lock().unwrap();
        let filtered: Vec<_> = rows
            .iter()
            .filter(|r| {
                let date = r.watched_at.date();
                date >= range.start() && date < range.end()
            })
            .filter(|r| match scope {
                crate::models::wrapup::WrapUpScope::User(uid) => r.user_id == *uid,
                crate::models::wrapup::WrapUpScope::Global => true,
            })
            .cloned()
            .collect();
        Ok(filtered)
    }
}

// ── InMemoryWrapUpRepository ────────────────────────────────────────────────

pub struct InMemoryWrapUpRepository {
    pub store: Mutex<Vec<crate::models::wrapup::WrapUpRecord>>,
}

impl InMemoryWrapUpRepository {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: Mutex::new(Vec::new()),
        })
    }
}

#[async_trait]
impl WrapUpRepository for InMemoryWrapUpRepository {
    async fn create(
        &self,
        record: &crate::models::wrapup::WrapUpRecord,
    ) -> Result<(), DomainError> {
        self.store.lock().unwrap().push(record.clone());
        Ok(())
    }

    async fn update_status(
        &self,
        id: &WrapUpId,
        status: &crate::models::wrapup::WrapUpStatus,
        error: Option<&str>,
    ) -> Result<(), DomainError> {
        let mut store = self.store.lock().unwrap();
        if let Some(rec) = store.iter_mut().find(|r| r.id == *id) {
            rec.status = status.clone();
            rec.error_message = error.map(|s| s.to_string());
            Ok(())
        } else {
            Err(DomainError::NotFound("wrapup record".into()))
        }
    }

    async fn set_complete(
        &self,
        id: &WrapUpId,
        report: &crate::models::wrapup::WrapUpReport,
    ) -> Result<(), DomainError> {
        let mut store = self.store.lock().unwrap();
        if let Some(rec) = store.iter_mut().find(|r| r.id == *id) {
            rec.status = crate::models::wrapup::WrapUpStatus::Ready;
            rec.report = Some(report.clone());
            rec.completed_at = Some(chrono::Utc::now().naive_utc());
            Ok(())
        } else {
            Err(DomainError::NotFound("wrapup record".into()))
        }
    }

    async fn get_by_id(
        &self,
        id: &WrapUpId,
    ) -> Result<Option<crate::models::wrapup::WrapUpRecord>, DomainError> {
        Ok(self
            .store
            .lock()
            .unwrap()
            .iter()
            .find(|r| r.id == *id)
            .cloned())
    }

    async fn list_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::wrapup::WrapUpRecord>, DomainError> {
        Ok(self
            .store
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.user_id == Some(user_id))
            .cloned()
            .collect())
    }

    async fn list_global(&self) -> Result<Vec<crate::models::wrapup::WrapUpRecord>, DomainError> {
        Ok(self
            .store
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.user_id.is_none())
            .cloned()
            .collect())
    }

    async fn find_existing(
        &self,
        user_id: Option<Uuid>,
        start: chrono::NaiveDate,
        end: chrono::NaiveDate,
    ) -> Result<Option<crate::models::wrapup::WrapUpRecord>, DomainError> {
        Ok(self
            .store
            .lock()
            .unwrap()
            .iter()
            .find(|r| r.user_id == user_id && r.start_date == start && r.end_date == end)
            .cloned())
    }

    async fn delete(&self, id: &WrapUpId) -> Result<(), DomainError> {
        self.store.lock().unwrap().retain(|r| r.id != *id);
        Ok(())
    }

    async fn delete_failed_older_than(
        &self,
        before: chrono::NaiveDateTime,
    ) -> Result<u64, DomainError> {
        let mut store = self.store.lock().unwrap();
        let before_len = store.len();
        store.retain(|r| {
            !(r.status == crate::models::wrapup::WrapUpStatus::Failed && r.created_at < before)
        });
        Ok((before_len - store.len()) as u64)
    }
}

// ── PanicWrapUpRepository ──────────────────────────────────────────────────

pub struct PanicWrapUpRepository;

#[async_trait]
impl WrapUpRepository for PanicWrapUpRepository {
    async fn create(&self, _: &crate::models::wrapup::WrapUpRecord) -> Result<(), DomainError> {
        panic!("PanicWrapUpRepository called")
    }
    async fn update_status(
        &self,
        _: &WrapUpId,
        _: &crate::models::wrapup::WrapUpStatus,
        _: Option<&str>,
    ) -> Result<(), DomainError> {
        panic!("PanicWrapUpRepository called")
    }
    async fn set_complete(
        &self,
        _: &WrapUpId,
        _: &crate::models::wrapup::WrapUpReport,
    ) -> Result<(), DomainError> {
        panic!("PanicWrapUpRepository called")
    }
    async fn get_by_id(
        &self,
        _: &WrapUpId,
    ) -> Result<Option<crate::models::wrapup::WrapUpRecord>, DomainError> {
        panic!("PanicWrapUpRepository called")
    }
    async fn list_for_user(
        &self,
        _: Uuid,
    ) -> Result<Vec<crate::models::wrapup::WrapUpRecord>, DomainError> {
        panic!("PanicWrapUpRepository called")
    }
    async fn list_global(&self) -> Result<Vec<crate::models::wrapup::WrapUpRecord>, DomainError> {
        panic!("PanicWrapUpRepository called")
    }
    async fn find_existing(
        &self,
        _: Option<Uuid>,
        _: chrono::NaiveDate,
        _: chrono::NaiveDate,
    ) -> Result<Option<crate::models::wrapup::WrapUpRecord>, DomainError> {
        panic!("PanicWrapUpRepository called")
    }
    async fn delete(&self, _: &WrapUpId) -> Result<(), DomainError> {
        panic!("PanicWrapUpRepository called")
    }
    async fn delete_failed_older_than(&self, _: chrono::NaiveDateTime) -> Result<u64, DomainError> {
        panic!("PanicWrapUpRepository called")
    }
}
