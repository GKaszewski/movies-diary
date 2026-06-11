use std::sync::Arc;

use async_trait::async_trait;
use domain::errors::DomainError;
use domain::testing::{FakeDiaryRepository, NoopSocialQueryPort};

use crate::{
    config::AppConfig,
    diary::deps::GetActivityFeedDeps,
    diary::get_activity_feed,
    diary::queries::GetActivityFeedQuery,
    test_helpers::TestContextBuilder,
};

fn default_deps() -> GetActivityFeedDeps {
    GetActivityFeedDeps {
        diary: FakeDiaryRepository::new() as _,
        social_query: Arc::new(NoopSocialQueryPort),
        config: TestContextBuilder::new().config,
    }
}

#[tokio::test]
async fn returns_empty_feed() {
    let deps = default_deps();

    let result = get_activity_feed::execute(
        &deps,
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
    let deps = default_deps();

    let viewer = uuid::Uuid::new_v4();

    let result = get_activity_feed::execute(
        &deps,
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

    let deps = GetActivityFeedDeps {
        diary: FakeDiaryRepository::new() as _,
        social_query: social as _,
        config: AppConfig {
            allow_registration: true,
            base_url: "http://localhost:3000".into(),
            rate_limit: 20,
            refresh_ttl_seconds: 2_592_000,
            wrapup: crate::config::WrapUpConfig {
                font_path: None,
                logo_path: None,
                bg_dir: None,
            },
        },
    };

    let result = get_activity_feed::execute(
        &deps,
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
    let deps = default_deps();

    let result = get_activity_feed::execute(
        &deps,
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
