use super::{
    movie::Movie,
    review::{DiaryEntry, Review},
};

#[derive(Debug, Clone, Default, PartialEq)]
pub enum FeedSortBy {
    #[default]
    Date,
    DateAsc,
    Rating,
    RatingAsc,
}

impl std::str::FromStr for FeedSortBy {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "date_asc" => Self::DateAsc,
            "rating" => Self::Rating,
            "rating_asc" => Self::RatingAsc,
            _ => Self::Date,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct FollowingFilter {
    pub local_user_ids: Vec<uuid::Uuid>,
    pub remote_actor_urls: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct FeedEntry {
    entry: DiaryEntry,
    user_email: String,
}

impl FeedEntry {
    pub fn new(entry: DiaryEntry, user_email: String) -> Self {
        Self { entry, user_email }
    }
    pub fn movie(&self) -> &Movie {
        self.entry.movie()
    }
    pub fn review(&self) -> &Review {
        self.entry.review()
    }
    pub fn user_email(&self) -> &str {
        &self.user_email
    }
    pub fn user_display_name(&self) -> &str {
        self.user_email
            .split('@')
            .next()
            .unwrap_or(&self.user_email)
    }
}
