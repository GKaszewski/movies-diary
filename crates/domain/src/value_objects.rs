use crate::errors::DomainError;
use uuid::Uuid;

macro_rules! uuid_id {
    ($name:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        pub struct $name(Uuid);

        impl $name {
            pub fn generate() -> Self {
                Self(Uuid::new_v4())
            }
            pub fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }
            pub fn value(&self) -> Uuid {
                self.0
            }
        }
    };
}

uuid_id!(MovieId);
uuid_id!(ReviewId);
uuid_id!(UserId);
uuid_id!(ImportSessionId);
uuid_id!(ImportProfileId);
uuid_id!(WatchlistEntryId);
uuid_id!(WatchEventId);
uuid_id!(WebhookTokenId);
uuid_id!(WrapUpId);
uuid_id!(GoalId);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalMetadataId(String);

impl ExternalMetadataId {
    pub fn new(id: String) -> Result<Self, DomainError> {
        let trimmed = id.trim();
        if trimmed.is_empty() {
            Err(DomainError::ValidationError(
                "External metadata ID cannot be empty".into(),
            ))
        } else {
            Ok(Self(trimmed.to_string()))
        }
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PosterPath(String);

impl PosterPath {
    pub fn new(path: String) -> Result<Self, DomainError> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            Err(DomainError::ValidationError(
                "Poster path cannot be empty".into(),
            ))
        } else {
            Ok(Self(trimmed.to_string()))
        }
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MovieTitle(String);

impl MovieTitle {
    const MAX_LENGTH: usize = 255;

    pub fn new(title: String) -> Result<Self, DomainError> {
        let trimmed = title.trim();
        if trimmed.is_empty() {
            Err(DomainError::ValidationError(
                "Movie title cannot be empty".into(),
            ))
        } else if trimmed.len() > Self::MAX_LENGTH {
            Err(DomainError::ValidationError(format!(
                "Movie title exceeds {} characters",
                Self::MAX_LENGTH
            )))
        } else {
            Ok(Self(trimmed.to_string()))
        }
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comment(String);

impl Comment {
    const MAX_LENGTH: usize = 10_000;

    pub fn new(comment: String) -> Result<Self, DomainError> {
        let trimmed = comment.trim();
        if trimmed.len() > Self::MAX_LENGTH {
            Err(DomainError::ValidationError(format!(
                "Comment exceeds {} characters",
                Self::MAX_LENGTH
            )))
        } else {
            Ok(Self(trimmed.to_string()))
        }
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rating(u8);

impl Rating {
    const MAX: u8 = 5;

    pub fn new(value: u8) -> Result<Self, DomainError> {
        if value <= Self::MAX {
            Ok(Self(value))
        } else {
            Err(DomainError::InvalidRating {
                max: Self::MAX,
                given: value,
            })
        }
    }
    pub fn value(&self) -> u8 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseYear(u16);

impl ReleaseYear {
    const EARLIEST: u16 = 1888;
    pub fn new(year: u16) -> Result<Self, DomainError> {
        if year < Self::EARLIEST {
            Err(DomainError::ValidationError(format!(
                "Release year cannot be earlier than {} (first film ever made)",
                Self::EARLIEST
            )))
        } else {
            Ok(Self(year))
        }
    }

    pub fn value(&self) -> u16 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Email(String);

impl Email {
    pub fn new(email: String) -> Result<Self, DomainError> {
        let trimmed = email.trim();
        if email_address::EmailAddress::is_valid(trimmed) {
            Ok(Self(trimmed.to_string()))
        } else {
            Err(DomainError::ValidationError("Invalid email format".into()))
        }
    }
    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Username(String);

impl Username {
    const MIN_LENGTH: usize = 2;
    const MAX_LENGTH: usize = 30;

    /// Accepts 2–30 chars: lowercase letters, digits, underscores, hyphens.
    /// Lowercases input automatically.
    pub fn new(raw: String) -> Result<Self, DomainError> {
        let s = raw.trim().to_lowercase();
        if s.len() < Self::MIN_LENGTH || s.len() > Self::MAX_LENGTH {
            return Err(DomainError::ValidationError(format!(
                "Username must be {}–{} characters",
                Self::MIN_LENGTH,
                Self::MAX_LENGTH
            )));
        }
        if !s
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(DomainError::ValidationError(
                "Username may only contain letters, digits, underscores, and hyphens".into(),
            ));
        }
        Ok(Self(s))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PasswordHash(String);

impl PasswordHash {
    pub fn new(hash: String) -> Result<Self, DomainError> {
        if hash.is_empty() {
            Err(DomainError::ValidationError(
                "Password hash cannot be empty".into(),
            ))
        } else {
            Ok(Self(hash))
        }
    }
    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PosterUrl(String);

impl PosterUrl {
    pub fn new(url: String) -> Result<Self, DomainError> {
        let trimmed = url.trim();
        if trimmed.is_empty() {
            Err(DomainError::ValidationError(
                "Poster URL cannot be empty".into(),
            ))
        } else {
            Ok(Self(trimmed.to_string()))
        }
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
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
}
