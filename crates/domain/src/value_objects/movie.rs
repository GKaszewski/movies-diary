use crate::errors::DomainError;

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
