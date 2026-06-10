use super::*;
use crate::value_objects::UserId;

fn make_goal(year: u16, target: u32) -> Result<Goal, DomainError> {
    Goal::new(UserId::generate(), year, target, GoalType::Movies)
}

#[test]
fn new_goal_valid() {
    let g = make_goal(2024, 52);
    assert!(g.is_ok());
    let g = g.unwrap();
    assert_eq!(g.year(), 2024);
    assert_eq!(g.target_count(), 52);
}

#[test]
fn new_goal_rejects_year_before_2020() {
    assert!(make_goal(2019, 10).is_err());
}

#[test]
fn new_goal_rejects_zero_target() {
    assert!(make_goal(2024, 0).is_err());
}

#[test]
fn update_target_valid() {
    let mut g = make_goal(2024, 10).unwrap();
    assert!(g.update_target(50).is_ok());
    assert_eq!(g.target_count(), 50);
}

#[test]
fn update_target_rejects_zero() {
    let mut g = make_goal(2024, 10).unwrap();
    assert!(g.update_target(0).is_err());
}

#[test]
fn from_persistence_preserves_fields() {
    let id = GoalId::generate();
    let uid = UserId::generate();
    let ts = chrono::Utc::now().naive_utc();
    let g = Goal::from_persistence(id.clone(), uid.clone(), 2025, 42, GoalType::Movies, ts);
    assert_eq!(*g.id(), id);
    assert_eq!(*g.user_id(), uid);
    assert_eq!(g.year(), 2025);
    assert_eq!(g.target_count(), 42);
    assert_eq!(g.created_at(), &ts);
}

#[test]
fn percentage_calculation() {
    let g = make_goal(2024, 100).unwrap();
    let wp = GoalWithProgress {
        goal: g,
        current_count: 50,
    };
    assert!((wp.percentage() - 50.0).abs() < f64::EPSILON);
}

#[test]
fn percentage_caps_at_100() {
    let g = make_goal(2024, 10).unwrap();
    let wp = GoalWithProgress {
        goal: g,
        current_count: 20,
    };
    assert!((wp.percentage() - 100.0).abs() < f64::EPSILON);
}

#[test]
fn is_complete() {
    let g = make_goal(2024, 10).unwrap();
    let wp = GoalWithProgress {
        goal: g,
        current_count: 10,
    };
    assert!(wp.is_complete());
}

#[test]
fn is_not_complete() {
    let g = make_goal(2024, 10).unwrap();
    let wp = GoalWithProgress {
        goal: g,
        current_count: 9,
    };
    assert!(!wp.is_complete());
}
