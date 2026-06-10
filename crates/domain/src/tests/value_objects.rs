use super::*;

#[test]
fn movie_id_generate_unique() {
    let a = MovieId::generate();
    let b = MovieId::generate();
    assert_ne!(a, b);
}

#[test]
fn rating_valid_range() {
    assert!(Rating::new(0).is_ok());
    assert!(Rating::new(5).is_ok());
    assert_eq!(Rating::new(3).unwrap().value(), 3);
}

#[test]
fn rating_invalid() {
    assert!(Rating::new(6).is_err());
    assert!(Rating::new(255).is_err());
}

#[test]
fn movie_title_valid() {
    let t = MovieTitle::new("Test".into());
    assert!(t.is_ok());
    assert_eq!(t.unwrap().value(), "Test");
}

#[test]
fn movie_title_empty_rejected() {
    assert!(MovieTitle::new("".into()).is_err());
    assert!(MovieTitle::new("   ".into()).is_err());
}

#[test]
fn release_year_valid() {
    assert!(ReleaseYear::new(2024).is_ok());
    assert_eq!(ReleaseYear::new(1888).unwrap().value(), 1888);
}

#[test]
fn release_year_too_early() {
    assert!(ReleaseYear::new(1887).is_err());
}

#[test]
fn email_valid() {
    let e = Email::new("a@b.com".into());
    assert!(e.is_ok());
    assert_eq!(e.unwrap().value(), "a@b.com");
}

#[test]
fn email_invalid() {
    assert!(Email::new("invalid".into()).is_err());
    assert!(Email::new("".into()).is_err());
}

#[test]
fn username_valid() {
    let u = Username::new("test".into());
    assert!(u.is_ok());
    assert_eq!(u.unwrap().value(), "test");
}

#[test]
fn username_lowercases() {
    assert_eq!(Username::new("Alice".into()).unwrap().value(), "alice");
}

#[test]
fn username_rejects_too_short() {
    assert!(Username::new("a".into()).is_err());
}

#[test]
fn username_rejects_special_chars() {
    assert!(Username::new("no spaces".into()).is_err());
    assert!(Username::new("no@at".into()).is_err());
}

#[test]
fn poster_path_valid() {
    let p = PosterPath::new("path/to/poster".into());
    assert!(p.is_ok());
    assert_eq!(p.unwrap().value(), "path/to/poster");
}

#[test]
fn poster_path_empty_rejected() {
    assert!(PosterPath::new("".into()).is_err());
}

#[test]
fn comment_valid() {
    let c = Comment::new("nice movie".into());
    assert!(c.is_ok());
    assert_eq!(c.unwrap().value(), "nice movie");
}

#[test]
fn comment_empty_is_ok() {
    // empty comment allowed — only max-length checked
    assert!(Comment::new("".into()).is_ok());
}

#[test]
fn external_metadata_id_valid() {
    let e = ExternalMetadataId::new("tt1234567".into());
    assert!(e.is_ok());
    assert_eq!(e.unwrap().value(), "tt1234567");
}

#[test]
fn external_metadata_id_empty_rejected() {
    assert!(ExternalMetadataId::new("".into()).is_err());
    assert!(ExternalMetadataId::new("   ".into()).is_err());
}

#[test]
fn password_hash_valid() {
    assert!(PasswordHash::new("hash".into()).is_ok());
}

#[test]
fn password_hash_empty_rejected() {
    assert!(PasswordHash::new("".into()).is_err());
}

#[test]
fn poster_url_valid() {
    let u = PosterUrl::new("https://img.com/poster.jpg".into());
    assert!(u.is_ok());
    assert_eq!(u.unwrap().value(), "https://img.com/poster.jpg");
}

#[test]
fn poster_url_empty_rejected() {
    assert!(PosterUrl::new("".into()).is_err());
}
