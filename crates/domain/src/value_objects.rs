use crate::errors::DomainError;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MovieId(Uuid);

impl MovieId {
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewId(Uuid);

impl ReviewId {
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserId(Uuid);

impl UserId {
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
    pub fn new(title: String) -> Result<Self, DomainError> {
        let trimmed = title.trim();
        if trimmed.is_empty() {
            Err(DomainError::ValidationError(
                "Movie title cannot be empty".into(),
            ))
        } else if trimmed.len() > 255 {
            Err(DomainError::ValidationError(
                "Movie title exceeds 255 characters".into(),
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
pub struct Comment(String);

impl Comment {
    pub fn new(comment: String) -> Result<Self, DomainError> {
        let trimmed = comment.trim();
        if trimmed.len() > 10_000 {
            Err(DomainError::ValidationError(
                "Comment exceeds 10,000 characters".into(),
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
