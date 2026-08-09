use crate::test_helpers::TestContextBuilder;
use crate::users::deps::GetFederatedProfileStatsDeps;
use crate::users::get_federated_profile_stats;
use crate::users::queries::{GetUserProfileQuery, ProfileView};

/// Mirrors exactly how `build_federated_profile_response`
/// (`crates/presentation/src/handlers/users.rs`) calls `execute`: a `user_id` that
/// has no row in the local `users` table (a federated/remote profile uses a
/// synthetic id — see `FederatedProfileQuery`), `is_own_profile: false`,
/// `include_remote: true`. Before ADR-0004's split this was the tolerant branch of
/// the unified `get_profile`; now it's simply this function's only behavior — it
/// never looks at the local `users` table at all, so there is nothing to be
/// tolerant about.
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
