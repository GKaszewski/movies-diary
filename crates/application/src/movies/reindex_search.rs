use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{IndexableDocument, MovieFilter, collections::PageParams},
    ports::EventHandler,
};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::movies::deps::ReindexSearchDeps;

const BATCH_SIZE: u32 = 500;

pub struct ReindexResult {
    pub movies_indexed: u64,
    pub persons_indexed: u64,
    pub persons_backfilled: u64,
}

pub async fn execute(deps: &ReindexSearchDeps) -> Result<ReindexResult, DomainError> {
    let movies_indexed = reindex_movies(deps).await?;
    let persons_backfilled = backfill_persons(deps).await?;
    let persons_indexed = reindex_persons(deps).await?;

    Ok(ReindexResult {
        movies_indexed,
        persons_indexed,
        persons_backfilled,
    })
}

async fn reindex_movies(deps: &ReindexSearchDeps) -> Result<u64, DomainError> {
    let mut count: u64 = 0;
    let mut offset: u32 = 0;
    loop {
        let page = deps
            .movie_query
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
            let profile = deps.movie_profile.get_by_movie_id(&movie_id).await?;

            if let Err(e) = deps
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

async fn backfill_persons(deps: &ReindexSearchDeps) -> Result<u64, DomainError> {
    let mut total = 0u64;
    loop {
        let (count, has_more) = deps
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

async fn reindex_persons(deps: &ReindexSearchDeps) -> Result<u64, DomainError> {
    let mut count: u64 = 0;
    let mut offset: u32 = 0;
    loop {
        let persons = deps.person_query.list_page(BATCH_SIZE, offset).await?;

        for person in &persons {
            if let Err(e) = deps
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

pub struct SearchReindexHandler {
    deps: ReindexSearchDeps,
    running: AtomicBool,
}

impl SearchReindexHandler {
    pub fn new(deps: ReindexSearchDeps) -> Self {
        Self {
            deps,
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

        tracing::info!("search reindex started");
        let result = execute(&self.deps).await;
        self.running.store(false, Ordering::SeqCst);

        let r = result?;
        if r.persons_backfilled > 0 {
            tracing::info!(
                backfilled = r.persons_backfilled,
                "backfilled missing persons from credits"
            );
        }
        tracing::info!(
            movies_indexed = r.movies_indexed,
            persons_indexed = r.persons_indexed,
            "search reindex completed"
        );
        Ok(())
    }
}
