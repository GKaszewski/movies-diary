use activitypub::objects::{WatchlistApInput, watchlist_to_ap_object};
use serde_json::json;

#[test]
fn watchlist_object_is_publicly_addressed_with_followers_in_cc() {
    let obj = watchlist_to_ap_object(WatchlistApInput {
        ap_id: "https://example.com/watchlist/1".parse().unwrap(),
        actor_url: "https://example.com/users/abc".parse().unwrap(),
        movie_title: "Alien".to_string(),
        release_year: 1979,
        external_metadata_id: None,
        poster_url: None,
        added_at: chrono::Utc::now(),
        base_url: "https://example.com".to_string(),
    });
    let json = serde_json::to_value(&obj).unwrap();

    assert_eq!(
        json["to"],
        json!(["https://www.w3.org/ns/activitystreams#Public"])
    );
    assert_eq!(
        json["cc"],
        json!(["https://example.com/users/abc/followers"])
    );
}
