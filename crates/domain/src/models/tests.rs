use super::*;
use crate::value_objects::{Email, PasswordHash, UserId, Username};

fn make_user() -> User {
    User::from_persistence(
        UserId::generate(),
        Email::new("a@b.com".to_string()).unwrap(),
        Username::new("alice".to_string()).unwrap(),
        PasswordHash::new("hash".to_string()).unwrap(),
        UserRole::Standard,
        UserProfile::default(),
    )
}

#[test]
fn update_profile_sets_fields() {
    let mut user = make_user();
    user.update_profile(UserProfile {
        bio: Some("My bio".to_string()),
        avatar_path: Some("avatars/abc".to_string()),
        ..Default::default()
    });
    assert_eq!(user.bio(), Some("My bio"));
    assert_eq!(user.avatar_path(), Some("avatars/abc"));
}

#[test]
fn update_profile_clears_with_none() {
    let mut user = make_user();
    user.update_profile(UserProfile {
        bio: Some("bio".to_string()),
        avatar_path: Some("path".to_string()),
        ..Default::default()
    });
    user.update_profile(UserProfile::default());
    assert_eq!(user.bio(), None);
    assert_eq!(user.avatar_path(), None);
}
