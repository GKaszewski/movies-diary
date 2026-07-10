use std::sync::Arc;

use async_trait::async_trait;
use domain::errors::DomainError;
use domain::testing::InMemorySocialRepository;
use domain::value_objects::SocialActor;

use crate::{
    config::AppConfig, diary::deps::GetActivityFeedDeps, diary::get_activity_feed,
    diary::queries::GetActivityFeedQuery, test_helpers::TestContextBuilder,
};

fn default_deps() -> GetActivityFeedDeps {
    GetActivityFeedDeps {
        diary: domain::testing::FakeDiaryQuery::new() as _,
        social_query: InMemorySocialRepository::new() as _,
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
            sort_by: domain::models::FeedSortBy::Date,
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
            sort_by: domain::models::FeedSortBy::Date,
            search: None,
            viewer_user_id: Some(viewer),
            filter_following: true,
        },
    )
    .await
    .unwrap();

    assert!(result.items.is_empty());
}

struct FakeSocialWithFollowing(Vec<String>);

#[async_trait]
impl domain::ports::SocialQuery for FakeSocialWithFollowing {
    async fn get_following(
        &self,
        _: &domain::value_objects::UserId,
    ) -> Result<Vec<SocialActor>, DomainError> {
        Ok(vec![])
    }
    async fn get_followers(
        &self,
        _: &domain::value_objects::UserId,
    ) -> Result<Vec<SocialActor>, DomainError> {
        Ok(vec![])
    }
    async fn get_pending_followers(
        &self,
        _: &domain::value_objects::UserId,
    ) -> Result<Vec<SocialActor>, DomainError> {
        Ok(vec![])
    }
    async fn count_following(
        &self,
        _: &domain::value_objects::UserId,
    ) -> Result<usize, DomainError> {
        Ok(0)
    }
    async fn count_followers(
        &self,
        _: &domain::value_objects::UserId,
    ) -> Result<usize, DomainError> {
        Ok(0)
    }
    async fn get_blocked(
        &self,
        _: &domain::value_objects::UserId,
    ) -> Result<Vec<SocialActor>, DomainError> {
        Ok(vec![])
    }
    async fn is_following(
        &self,
        _: &domain::value_objects::UserId,
        _: &domain::value_objects::SocialIdentity,
    ) -> Result<bool, DomainError> {
        Ok(false)
    }
    async fn get_accepted_following_urls(
        &self,
        _: &domain::value_objects::UserId,
    ) -> Result<Vec<String>, DomainError> {
        Ok(self.0.clone())
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
        diary: domain::testing::FakeDiaryQuery::new() as _,
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
            sort_by: domain::models::FeedSortBy::Date,
            search: None,
            viewer_user_id: Some(viewer),
            filter_following: true,
        },
    )
    .await
    .unwrap();

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
            sort_by: domain::models::FeedSortBy::Date,
            search: None,
            viewer_user_id: None,
            filter_following: true,
        },
    )
    .await
    .unwrap();

    assert!(result.items.is_empty());
}
