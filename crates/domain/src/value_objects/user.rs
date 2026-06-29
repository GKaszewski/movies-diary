use crate::errors::DomainError;

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

#[derive(Clone, PartialEq, Eq)]
pub struct Password(String);

impl std::fmt::Debug for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Password([REDACTED])")
    }
}

impl Password {
    const MIN_LENGTH: usize = 8;

    pub fn new(raw: String) -> Result<Self, DomainError> {
        if raw.chars().count() < Self::MIN_LENGTH {
            Err(DomainError::ValidationError(
                "Password must be at least 8 characters".into(),
            ))
        } else {
            Ok(Self(raw))
        }
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}
