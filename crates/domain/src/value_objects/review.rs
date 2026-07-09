use std::fmt;
use std::str::FromStr;

use crate::errors::DomainError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WatchMedium {
    Cinema,
    Streaming,
    TV,
    PhysicalMedia,
    Download,
    MediaServer,
    Other,
}

impl fmt::Display for WatchMedium {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cinema => write!(f, "cinema"),
            Self::Streaming => write!(f, "streaming"),
            Self::TV => write!(f, "tv"),
            Self::PhysicalMedia => write!(f, "physical_media"),
            Self::Download => write!(f, "download"),
            Self::MediaServer => write!(f, "media_server"),
            Self::Other => write!(f, "other"),
        }
    }
}

impl FromStr for WatchMedium {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "cinema" => Ok(Self::Cinema),
            "streaming" => Ok(Self::Streaming),
            "tv" => Ok(Self::TV),
            "physical_media" => Ok(Self::PhysicalMedia),
            "download" => Ok(Self::Download),
            "media_server" => Ok(Self::MediaServer),
            "other" => Ok(Self::Other),
            _ => Err(DomainError::ValidationError(format!(
                "unknown watch medium: {s}"
            ))),
        }
    }
}

pub fn parse_watched_at(s: &str) -> Result<chrono::NaiveDateTime, DomainError> {
    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M"))
        .or_else(|_| {
            chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map(|d| d.and_hms_opt(0, 0, 0).expect("midnight always valid"))
        })
        .map_err(|_| {
            DomainError::ValidationError(format!(
                "invalid date '{s}'; expected YYYY-MM-DD or YYYY-MM-DDTHH:MM[:SS]"
            ))
        })
}

pub fn format_watched_at(dt: &chrono::NaiveDateTime) -> String {
    dt.format("%Y-%m-%dT%H:%M:%S").to_string()
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
