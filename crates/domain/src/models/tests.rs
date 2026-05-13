use super::*;
use crate::value_objects::{Email, PasswordHash, UserId, Username};

fn make_user() -> User {
    User::from_persistence(
        UserId::generate(),
        Email::new("a@b.com".to_string()).unwrap(),
        Username::new("alice".to_string()).unwrap(),
        PasswordHash::new("hash".to_string()).unwrap(),
        UserRole::Standard,
        None,
        None,
        None,
        None,
        vec![],
    )
}

#[test]
fn update_profile_sets_fields() {
    let mut user = make_user();
    user.update_profile(
        Some("My bio".to_string()),
        Some("avatars/abc".to_string()),
        None,
        None,
    );
    assert_eq!(user.bio(), Some("My bio"));
    assert_eq!(user.avatar_path(), Some("avatars/abc"));
}

#[test]
fn update_profile_clears_with_none() {
    let mut user = make_user();
    user.update_profile(
        Some("bio".to_string()),
        Some("path".to_string()),
        None,
        None,
    );
    user.update_profile(None, None, None, None);
    assert_eq!(user.bio(), None);
    assert_eq!(user.avatar_path(), None);
}
