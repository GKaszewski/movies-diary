use domain::testing::{test_review, test_watched_at};
use domain::value_objects::{Comment, MovieId, Rating, UserId, WatchMedium};

use domain::models::*;

fn review_with_medium(watch_medium: Option<WatchMedium>) -> Review {
    test_review(&MovieId::generate(), &UserId::generate(), 4).apply_edit(ReviewEdit {
        comment: Some(Some(Comment::new("great film".into()).unwrap())),
        watch_medium: Some(watch_medium),
        ..Default::default()
    })
}

fn new_review_with_medium(watch_medium: Option<WatchMedium>) -> Review {
    Review::new(
        MovieId::generate(),
        UserId::generate(),
        Rating::new(4).unwrap(),
        Some(Comment::new("great film".into()).unwrap()),
        test_watched_at(),
        watch_medium,
        false,
    )
    .unwrap()
}

#[test]
fn new_review_stores_watch_medium() {
    let review = new_review_with_medium(Some(WatchMedium::Cinema));
    assert_eq!(review.watch_medium(), Some(&WatchMedium::Cinema));
}

#[test]
fn new_review_without_medium() {
    let review = new_review_with_medium(None);
    assert_eq!(review.watch_medium(), None);
}

#[test]
fn apply_edit_updates_rating_only() {
    let original = review_with_medium(Some(WatchMedium::Streaming));
    let edited = original.apply_edit(ReviewEdit {
        rating: Some(Rating::new(2).unwrap()),
        ..Default::default()
    });
    assert_eq!(edited.rating().value(), 2);
    assert_eq!(edited.comment().unwrap().value(), "great film");
    assert_eq!(edited.watch_medium(), Some(&WatchMedium::Streaming));
}

#[test]
fn apply_edit_clears_comment() {
    let original = review_with_medium(None);
    let edited = original.apply_edit(ReviewEdit {
        comment: Some(None),
        ..Default::default()
    });
    assert!(edited.comment().is_none());
    assert_eq!(edited.rating().value(), 4);
}

#[test]
fn apply_edit_sets_watch_medium() {
    let original = review_with_medium(None);
    let edited = original.apply_edit(ReviewEdit {
        watch_medium: Some(Some(WatchMedium::Cinema)),
        ..Default::default()
    });
    assert_eq!(edited.watch_medium(), Some(&WatchMedium::Cinema));
}

#[test]
fn apply_edit_clears_watch_medium() {
    let original = review_with_medium(Some(WatchMedium::Cinema));
    let edited = original.apply_edit(ReviewEdit {
        watch_medium: Some(None),
        ..Default::default()
    });
    assert_eq!(edited.watch_medium(), None);
}

#[test]
fn apply_edit_toggles_spoilers() {
    let original = review_with_medium(None);
    let flag_on = original.apply_edit(ReviewEdit {
        spoilers: Some(true),
        ..Default::default()
    });
    assert!(flag_on.spoilers());
    let flag_off = flag_on.apply_edit(ReviewEdit {
        spoilers: Some(false),
        ..Default::default()
    });
    assert!(!flag_off.spoilers());
    let untouched = original.apply_edit(ReviewEdit::default());
    assert!(!untouched.spoilers());
}

#[test]
fn apply_edit_no_changes_preserves_all() {
    let original = review_with_medium(Some(WatchMedium::TV));
    let edited = original.apply_edit(ReviewEdit::default());
    assert_eq!(edited.rating().value(), 4);
    assert_eq!(edited.comment().unwrap().value(), "great film");
    assert_eq!(edited.watch_medium(), Some(&WatchMedium::TV));
}
