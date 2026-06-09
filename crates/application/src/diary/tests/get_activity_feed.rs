use std::sync::Arc;

use async_trait::async_trait;
use domain::errors::DomainError;

use crate::{
    diary::get_activity_feed, diary::queries::GetActivityFeedQuery,
    test_helpers::TestContextBuilder,
};

#[tokio::test]
async fn returns_empty_feed() {
    let ctx = TestContextBuilder::new().build();

    let result = get_activity_feed::execute(
        &ctx,
        GetActivityFeedQuery {
            limit: 10,
            offset: 0,
            sort_by: domain::ports::FeedSortBy::Date,
            search: None,
            viewer_user_id: None,
            filter_following: false,
        },
    )
    .await
    .unwrap();

    assert!(result.items.is_empty());
    assert_eq!(result.total_count, 0);
}

#[tokio::test]
async fn returns_feed_with_following_filter() {
    let ctx = TestContextBuilder::new().build();

    let viewer = uuid::Uuid::new_v4();

    let result = get_activity_feed::execute(
        &ctx,
        GetActivityFeedQuery {
            limit: 10,
            offset: 0,
            sort_by: domain::ports::FeedSortBy::Date,
            search: None,
            viewer_user_id: Some(viewer),
            filter_following: true,
        },
    )
    .await
    .unwrap();

    // NoopSocialQueryPort returns empty following, so FollowingFilter
    // contains only the viewer's id. Feed is empty but the code path is hit.
    assert!(result.items.is_empty());
}

struct FakeSocialWithFollowing(Vec<String>);

#[async_trait]
impl domain::ports::SocialQueryPort for FakeSocialWithFollowing {
    async fn get_accepted_following_urls(&self, _: uuid::Uuid) -> Result<Vec<String>, DomainError> {
        Ok(self.0.clone())
    }
    async fn count_following(&self, _: uuid::Uuid) -> Result<usize, DomainError> {
        Ok(0)
    }
    async fn count_accepted_followers(&self, _: uuid::Uuid) -> Result<usize, DomainError> {
        Ok(0)
    }
    async fn get_pending_followers(
        &self,
        _: uuid::Uuid,
    ) -> Result<Vec<domain::ports::PendingFollowerInfo>, DomainError> {
        Ok(vec![])
    }
    async fn list_all_followed_remote_actors(
        &self,
    ) -> Result<Vec<domain::ports::RemoteActorInfo>, DomainError> {
        Ok(vec![])
    }
}

#[tokio::test]
async fn following_filter_parses_local_and_remote_urls() {
    let viewer = uuid::Uuid::new_v4();
    let local_friend = uuid::Uuid::new_v4();

    let following_urls = vec![
        format!("http://localhost:3000/users/{}", local_friend),
        "https://remote.example/actor/1".to_string(),
    ];

    let social = Arc::new(FakeSocialWithFollowing(following_urls));

    let ctx = TestContextBuilder::new()
        .with_social_query(social as _)
        .build();

    let result = get_activity_feed::execute(
        &ctx,
        GetActivityFeedQuery {
            limit: 10,
            offset: 0,
            sort_by: domain::ports::FeedSortBy::Date,
            search: None,
            viewer_user_id: Some(viewer),
            filter_following: true,
        },
    )
    .await
    .unwrap();

    // Feed is empty (no data seeded), but the build_following_filter code path
    // with actual URL parsing ran without errors.
    assert!(result.items.is_empty());
}

#[tokio::test]
async fn following_filter_without_viewer_returns_none() {
    let ctx = TestContextBuilder::new().build();

    let result = get_activity_feed::execute(
        &ctx,
        GetActivityFeedQuery {
            limit: 10,
            offset: 0,
            sort_by: domain::ports::FeedSortBy::Date,
            search: None,
            viewer_user_id: None,
            filter_following: true,
        },
    )
    .await
    .unwrap();

    // filter_following=true but viewer_user_id=None → build_following_filter returns None
    assert!(result.items.is_empty());
}
