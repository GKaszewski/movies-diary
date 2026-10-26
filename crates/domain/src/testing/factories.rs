use chrono::{NaiveDate, NaiveDateTime};

use crate::models::{
    DiaryEntry, ExternalPersonId, Goal, GoalType, Movie, Person, Review, User, UserProfile,
    UserRole,
};
use crate::value_objects::{
    Email, ExternalMetadataId, MovieId, MovieTitle, PasswordHash, PersonId, Rating, ReleaseYear,
    UserId, Username,
};

const DEFAULT_YEAR: u16 = 2000;

pub fn test_watched_at() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2024, 3, 15)
        .unwrap()
        .and_hms_opt(20, 0, 0)
        .unwrap()
}

pub fn test_movie(title: &str) -> Movie {
    test_movie_with_year(title, DEFAULT_YEAR)
}

pub fn test_movie_with_year(title: &str, year: u16) -> Movie {
    Movie::new(
        None,
        MovieTitle::new(title.to_string()).unwrap(),
        ReleaseYear::new(year).unwrap(),
        None,
        None,
    )
}

pub fn test_movie_with_director(title: &str, year: u16, director: &str) -> Movie {
    Movie::new(
        None,
        MovieTitle::new(title.to_string()).unwrap(),
        ReleaseYear::new(year).unwrap(),
        Some(director.to_string()),
        None,
    )
}

pub fn test_movie_with_external_id(
    title: &str,
    year: u16,
    external_id: ExternalMetadataId,
) -> Movie {
    Movie::new(
        Some(external_id),
        MovieTitle::new(title.to_string()).unwrap(),
        ReleaseYear::new(year).unwrap(),
        None,
        None,
    )
}

pub fn test_review(movie_id: &MovieId, user_id: &UserId, rating: u8) -> Review {
    build_review(movie_id, user_id, rating, false)
}

pub fn test_spoiler_review(movie_id: &MovieId, user_id: &UserId, rating: u8) -> Review {
    build_review(movie_id, user_id, rating, true)
}

fn build_review(movie_id: &MovieId, user_id: &UserId, rating: u8, spoilers: bool) -> Review {
    Review::new(
        movie_id.clone(),
        user_id.clone(),
        Rating::new(rating).unwrap(),
        None,
        test_watched_at(),
        None,
        spoilers,
    )
    .unwrap()
}

pub fn test_user(username: &str) -> User {
    User::new(
        Email::new(format!("{username}@example.com")).unwrap(),
        Username::new(username.to_string()).unwrap(),
        PasswordHash::new("hashed-password".to_string()).unwrap(),
        UserRole::Standard,
    )
}

pub fn test_diary_entry(title: &str, rating: u8) -> DiaryEntry {
    let movie = test_movie(title);
    let review = test_review(movie.id(), &UserId::generate(), rating);
    DiaryEntry::new(movie, review)
}

pub fn test_goal(user_id: &UserId, year: u16, target_count: u32) -> Goal {
    Goal::new(user_id.clone(), year, target_count, GoalType::Movies).unwrap()
}

pub fn test_user_with_role(id: UserId, role: UserRole) -> User {
    User::from_persistence(
        id,
        Email::new("role-test@example.com".to_string()).unwrap(),
        Username::new("role_test_user".to_string()).unwrap(),
        PasswordHash::new("hashed-password".to_string()).unwrap(),
        role,
        UserProfile::default(),
    )
}

pub fn test_person(name: &str) -> Person {
    let ext = ExternalPersonId::new(format!("test:{name}"));
    Person::basic(
        PersonId::from_external(&ext),
        ext,
        name.to_string(),
        None,
        None,
    )
}

pub fn test_person_with_tmdb(tmdb_id: i64, name: &str, department: Option<&str>) -> Person {
    let ext = ExternalPersonId::new(format!("tmdb:{tmdb_id}"));
    Person::basic(
        PersonId::from_external(&ext),
        ext,
        name.to_string(),
        department.map(str::to_string),
        None,
    )
}
