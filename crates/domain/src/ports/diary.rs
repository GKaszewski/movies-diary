use async_trait::async_trait;

use crate::{
    errors::DomainError,
    events::DomainEvent,
    models::{
        DiaryEntry, DiaryFilter, ExportFormat, FeedEntry, FeedSortBy, FollowingFilter, MovieStats,
        Review, ReviewHistory, UserStats, UserTrends,
        collections::{PageParams, Paginated},
    },
    value_objects::{MovieId, ReviewId, UserId},
};

#[async_trait]
pub trait DiaryRepository: Send + Sync {
    async fn query_diary(&self, filter: &DiaryFilter)
    -> Result<Paginated<DiaryEntry>, DomainError>;
    async fn query_activity_feed(
        &self,
        page: &PageParams,
    ) -> Result<Paginated<FeedEntry>, DomainError>;
    async fn query_activity_feed_filtered(
        &self,
        page: &PageParams,
        sort_by: &FeedSortBy,
        search: Option<&str>,
        following: Option<&FollowingFilter>,
    ) -> Result<Paginated<FeedEntry>, DomainError>;
    async fn get_review_history(&self, movie_id: &MovieId) -> Result<ReviewHistory, DomainError>;
    async fn get_user_history(&self, user_id: &UserId) -> Result<Vec<DiaryEntry>, DomainError>;
    fn stream_user_history(
        &self,
        user_id: UserId,
    ) -> futures::stream::BoxStream<'static, Result<DiaryEntry, DomainError>>;
    async fn get_movie_stats(&self, movie_id: &MovieId) -> Result<MovieStats, DomainError>;
    async fn get_movie_social_feed(
        &self,
        movie_id: &MovieId,
        page: &PageParams,
    ) -> Result<Paginated<FeedEntry>, DomainError>;
    async fn count_local_posts(&self) -> Result<u64, DomainError>;
}

#[async_trait]
pub trait ReviewRepository: Send + Sync {
    async fn save_review(&self, review: &Review) -> Result<DomainEvent, DomainError>;
    async fn get_review_by_id(&self, review_id: &ReviewId) -> Result<Option<Review>, DomainError>;
    async fn delete_review(&self, review_id: &ReviewId) -> Result<(), DomainError>;
    async fn get_all_reviews_for_user(&self, user_id: &UserId) -> Result<Vec<Review>, DomainError>;
}

#[async_trait]
pub trait StatsRepository: Send + Sync {
    async fn get_user_stats(&self, user_id: &UserId) -> Result<UserStats, DomainError>;
    async fn get_user_trends(&self, user_id: &UserId) -> Result<UserTrends, DomainError>;
}

pub trait DiaryExporter: Send + Sync {
    fn stream_entries(
        &self,
        stream: futures::stream::BoxStream<'static, Result<DiaryEntry, DomainError>>,
        format: ExportFormat,
    ) -> futures::stream::BoxStream<'static, Result<bytes::Bytes, DomainError>>;
}
