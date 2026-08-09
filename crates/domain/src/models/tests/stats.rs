use super::*;

fn appearance(person_id: u64, name: &str, billing: u32, movie: &str) -> ActorAppearance {
    ActorAppearance {
        tmdb_person_id: person_id,
        name: name.to_string(),
        profile_path: Some(format!("/profile_{person_id}.jpg")),
        billing_order: billing,
        movie_id: movie.to_string(),
    }
}

#[test]
fn empty_appearances_returns_empty() {
    assert!(compute_top_actors(vec![]).is_empty());
}

#[test]
fn lead_scores_higher_than_minor_with_same_movie_count() {
    let appearances = vec![
        appearance(1, "Lead Actor", 0, "movie_a"),
        appearance(1, "Lead Actor", 1, "movie_b"),
        appearance(2, "Minor Actor", 15, "movie_a"),
        appearance(2, "Minor Actor", 12, "movie_b"),
    ];
    let result = compute_top_actors(appearances);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].name, "Lead Actor");
    assert_eq!(result[0].movie_count, 2);
    assert!(result[0].score > result[1].score);
}

#[test]
fn same_movie_counted_once_per_actor() {
    let appearances = vec![
        appearance(1, "Actor A", 0, "movie_a"),
        appearance(1, "Actor A", 0, "movie_a"),
    ];
    let result = compute_top_actors(appearances);
    assert_eq!(result[0].movie_count, 1);
}

#[test]
fn returns_at_most_five() {
    let appearances: Vec<ActorAppearance> = (0..10)
        .flat_map(|i| {
            (0..=i)
                .map(move |m| appearance(i as u64, &format!("Actor {i}"), 0, &format!("movie_{m}")))
        })
        .collect();
    let result = compute_top_actors(appearances);
    assert_eq!(result.len(), 5);
}

#[test]
fn tier_weights_applied_correctly() {
    let appearances = vec![
        appearance(1, "Lead", 0, "m1"),
        appearance(2, "Support", 5, "m1"),
        appearance(2, "Support", 3, "m2"),
        appearance(3, "Minor", 10, "m1"),
    ];
    let result = compute_top_actors(appearances);
    assert_eq!(result[0].name, "Support");
    assert!((result[0].score - 1.4).abs() < 0.001);
    assert_eq!(result[0].movie_count, 2);
    assert_eq!(result[1].name, "Lead");
    assert!((result[1].score - 1.0).abs() < 0.001);
    assert_eq!(result[2].name, "Minor");
    assert!((result[2].score - 0.2).abs() < 0.001);
}
