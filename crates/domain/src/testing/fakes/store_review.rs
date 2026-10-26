use async_trait::async_trait;

use crate::{
    errors::DomainError,
    events::DomainEvent,
    models::Review,
    ports::ReviewRepository,
    value_objects::{MovieId, ReviewId, UserId},
};

use super::InMemoryStore;

#[async_trait]
impl ReviewRepository for InMemoryStore {
    async fn save_review(
        &self,
        review: &Review,
        events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        self.reviews
            .write()
            .unwrap()
            .insert(review.id().value(), review.clone());
        self.record_events(events);
        Ok(())
    }

    async fn get_review_by_id(&self, review_id: &ReviewId) -> Result<Option<Review>, DomainError> {
        Ok(self
            .reviews
            .read()
            .unwrap()
            .get(&review_id.value())
            .cloned())
    }

    async fn update_review(
        &self,
        review: &Review,
        events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        self.reviews
            .write()
            .unwrap()
            .insert(review.id().value(), review.clone());
        self.record_events(events);
        Ok(())
    }

    async fn delete_review_and_orphaned_movie(
        &self,
        review_id: &ReviewId,
        movie_id: &MovieId,
        review_events: &[DomainEvent],
        orphan_events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        let movie = movie_id.value();
        let still_referenced = {
            let mut reviews = self.reviews.write().unwrap();
            reviews.remove(&review_id.value());
            reviews.values().any(|r| r.movie_id().value() == movie)
        };
        self.record_events(review_events);
        if !still_referenced && self.movies.read().unwrap().contains_key(&movie) {
            self.remove_movie_with_cascades(movie);
            self.record_events(orphan_events);
        }
        Ok(())
    }

    async fn get_all_reviews_for_user(&self, user_id: &UserId) -> Result<Vec<Review>, DomainError> {
        let store = self.reviews.read().unwrap();
        Ok(store
            .values()
            .filter(|r| r.user_id() == user_id)
            .cloned()
            .collect())
    }
}
