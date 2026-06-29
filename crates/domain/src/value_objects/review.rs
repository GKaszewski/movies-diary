use crate::errors::DomainError;

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
