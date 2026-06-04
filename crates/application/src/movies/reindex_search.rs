use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{IndexableDocument, MovieFilter, collections::PageParams},
    ports::EventHandler,
};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::context::AppContext;

const BATCH_SIZE: u32 = 500;

pub struct SearchReindexHandler {
    ctx: AppContext,
    running: AtomicBool,
}

impl SearchReindexHandler {
    pub fn new(ctx: AppContext) -> Self {
        Self {
            ctx,
            running: AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl EventHandler for SearchReindexHandler {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError> {
        if !matches!(event, DomainEvent::SearchReindexRequested) {
            return Ok(());
        }

        if self.running.swap(true, Ordering::SeqCst) {
            tracing::info!("search reindex already running, skipping");
            return Ok(());
        }

        let result = self.run_reindex().await;
        self.running.store(false, Ordering::SeqCst);
        result
    }
}

impl SearchReindexHandler {
    async fn run_reindex(&self) -> Result<(), DomainError> {
        tracing::info!("search reindex started");

        let movies_indexed = self.reindex_movies().await?;
        let backfilled = self.backfill_persons().await?;
        if backfilled > 0 {
            tracing::info!(backfilled, "backfilled missing persons from credits");
        }
        let persons_indexed = self.reindex_persons().await?;

        tracing::info!(movies_indexed, persons_indexed, "search reindex completed");
        Ok(())
    }

    async fn reindex_movies(&self) -> Result<u64, DomainError> {
        let mut count: u64 = 0;
        let mut offset: u32 = 0;
        loop {
            let page = self
                .ctx
                .repos
                .movie
                .list_movies(
                    &PageParams {
                        limit: BATCH_SIZE,
                        offset,
                    },
                    &MovieFilter::default(),
                )
                .await?;

            for summary in &page.items {
                let movie_id = summary.movie.id().clone();
                let profile = self
                    .ctx
                    .repos
                    .movie_profile
                    .get_by_movie_id(&movie_id)
                    .await?;

                if let Err(e) = self
                    .ctx
                    .repos
                    .search_command
                    .index(IndexableDocument::Movie {
                        id: movie_id.clone(),
                        movie: Box::new(summary.movie.clone()),
                        profile: profile.map(Box::new),
                    })
                    .await
                {
                    tracing::warn!(movie_id = %movie_id.value(), "reindex movie failed: {e}");
                }
                count += 1;
            }

            if (page.items.len() as u32) < BATCH_SIZE {
                break;
            }
            offset += BATCH_SIZE;
            tokio::task::yield_now().await;
        }
        Ok(count)
    }

    async fn backfill_persons(&self) -> Result<u64, DomainError> {
        let mut total = 0u64;
        loop {
            let (count, has_more) = self
                .ctx
                .repos
                .person_command
                .backfill_from_credits_batch(BATCH_SIZE)
                .await?;
            total += count;
            if !has_more {
                break;
            }
            tokio::task::yield_now().await;
        }
        Ok(total)
    }

    async fn reindex_persons(&self) -> Result<u64, DomainError> {
        let mut count: u64 = 0;
        let mut offset: u32 = 0;
        loop {
            let persons = self
                .ctx
                .repos
                .person_query
                .list_page(BATCH_SIZE, offset)
                .await?;

            for person in &persons {
                if let Err(e) = self
                    .ctx
                    .repos
                    .search_command
                    .index(IndexableDocument::Person {
                        id: person.id().clone(),
                        person: Box::new(person.clone()),
                    })
                    .await
                {
                    tracing::warn!(person = %person.name(), "reindex person failed: {e}");
                }
                count += 1;
            }

            if (persons.len() as u32) < BATCH_SIZE {
                break;
            }
            offset += BATCH_SIZE;
            tokio::task::yield_now().await;
        }
        Ok(count)
    }
}
