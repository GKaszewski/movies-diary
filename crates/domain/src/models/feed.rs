use super::{
    movie::Movie,
    review::{DiaryEntry, Review},
};

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
