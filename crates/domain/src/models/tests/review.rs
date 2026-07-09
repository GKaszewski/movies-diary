use chrono::NaiveDateTime;

use crate::value_objects::{Comment, MovieId, Rating, UserId, WatchMedium};

use super::*;

fn make_review(watch_medium: Option<WatchMedium>) -> Review {
    Review::new(
        MovieId::generate(),
        UserId::generate(),
        Rating::new(4).unwrap(),
        Some(Comment::new("great film".into()).unwrap()),
        NaiveDateTime::parse_from_str("2024-06-15 20:00:00", "%Y-%m-%d %H:%M:%S").unwrap(),
        watch_medium,
    )
    .unwrap()
}

#[test]
fn new_review_stores_watch_medium() {
    let review = make_review(Some(WatchMedium::Cinema));
    assert_eq!(review.watch_medium(), Some(&WatchMedium::Cinema));
}

#[test]
fn new_review_without_medium() {
    let review = make_review(None);
    assert_eq!(review.watch_medium(), None);
}

#[test]
fn apply_edit_updates_rating_only() {
    let original = make_review(Some(WatchMedium::Streaming));
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
    let original = make_review(None);
    let edited = original.apply_edit(ReviewEdit {
        comment: Some(None),
        ..Default::default()
    });
    assert!(edited.comment().is_none());
    assert_eq!(edited.rating().value(), 4);
}

#[test]
fn apply_edit_sets_watch_medium() {
    let original = make_review(None);
    let edited = original.apply_edit(ReviewEdit {
        watch_medium: Some(Some(WatchMedium::Cinema)),
        ..Default::default()
    });
    assert_eq!(edited.watch_medium(), Some(&WatchMedium::Cinema));
}

#[test]
fn apply_edit_clears_watch_medium() {
    let original = make_review(Some(WatchMedium::Cinema));
    let edited = original.apply_edit(ReviewEdit {
        watch_medium: Some(None),
        ..Default::default()
    });
    assert_eq!(edited.watch_medium(), None);
}

#[test]
fn apply_edit_no_changes_preserves_all() {
    let original = make_review(Some(WatchMedium::TV));
    let edited = original.apply_edit(ReviewEdit::default());
    assert_eq!(edited.rating().value(), 4);
    assert_eq!(edited.comment().unwrap().value(), "great film");
    assert_eq!(edited.watch_medium(), Some(&WatchMedium::TV));
}
