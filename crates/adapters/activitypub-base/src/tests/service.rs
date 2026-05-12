use super::*;
use crate::repository::{Follower, FollowerStatus, RemoteActor};

fn make_follower(inbox: &str, shared: Option<&str>) -> Follower {
    Follower {
        actor: RemoteActor {
            url: format!("https://remote/{}", inbox),
            handle: "user".to_string(),
            inbox_url: inbox.to_string(),
            shared_inbox_url: shared.map(|s| s.to_string()),
            display_name: None,
            avatar_url: None,
        },
        status: FollowerStatus::Accepted,
    }
}

#[test]
fn collect_inboxes_deduplicates_shared() {
    let followers = vec![
        make_follower("https://mastodon.social/users/a/inbox", Some("https://mastodon.social/inbox")),
        make_follower("https://mastodon.social/users/b/inbox", Some("https://mastodon.social/inbox")),
        make_follower("https://other.instance/users/c/inbox", None),
    ];
    let inboxes = collect_inboxes(&followers);
    assert_eq!(inboxes.len(), 2);
    let strs: Vec<_> = inboxes.iter().map(|u| u.as_str()).collect();
    assert!(strs.contains(&"https://mastodon.social/inbox"));
    assert!(strs.contains(&"https://other.instance/users/c/inbox"));
}

#[test]
fn collect_inboxes_falls_back_to_individual_inbox() {
    let followers = vec![
        make_follower("https://example.com/users/x/inbox", None),
    ];
    let inboxes = collect_inboxes(&followers);
    assert_eq!(inboxes.len(), 1);
    assert_eq!(inboxes[0].as_str(), "https://example.com/users/x/inbox");
}
