use crate::support::TestContextBuilder;
use application::users::deps::GetFederatedProfileStatsDeps;
use application::users::get_federated_profile_stats;
use application::users::queries::{GetUserProfileQuery, ProfileView};

#[tokio::test]
async fn succeeds_for_a_user_id_with_no_local_row() {
    let b = TestContextBuilder::new();
    let deps = GetFederatedProfileStatsDeps {
        stats: b.stats_repo.clone(),
        diary: b.diary_repo.clone(),
        social_query: b.social_query_unified.clone(),
    };

    let synthetic_user_id = uuid::Uuid::new_v4();

    let result = get_federated_profile_stats::execute(
        &deps,
        GetUserProfileQuery {
            user_id: synthetic_user_id,
            view: ProfileView::Recent,
            limit: None,
            offset: None,
            sort_by: domain::models::FeedSortBy::Date,
            search: None,
            is_own_profile: false,
            include_remote: true,
        },
    )
    .await;

    assert!(
        result.is_ok(),
        "execute must not hard-fail for a user_id absent from the local `users` table \
         (the federated-profile call shape) — got Err"
    );
}
