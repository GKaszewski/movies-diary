use domain::{
    errors::DomainError, models::DiaryEntry, models::ReviewSortBy, value_objects::UserId,
};
use uuid::Uuid;

use crate::diary::deps::{GetDiaryDeps, GetUserFeedDeps};
use crate::diary::get_diary;
use crate::diary::queries::GetDiaryQuery;

pub struct FeedAuthor {
    pub display_name: String,
}

pub struct UserFeed {
    pub author: FeedAuthor,
    pub entries: Vec<DiaryEntry>,
}

pub async fn execute(
    deps: &GetUserFeedDeps,
    user_id: Uuid,
    limit: u32,
) -> Result<UserFeed, DomainError> {
    let user = deps
        .user
        .find_by_id(&UserId::from_uuid(user_id))
        .await?
        .ok_or_else(|| DomainError::NotFound(format!("User {user_id}")))?;

    let query = GetDiaryQuery {
        limit: Some(limit),
        offset: Some(0),
        sort_by: Some(ReviewSortBy::Descending),
        movie_id: None,
        user_id: Some(user_id),
    };
    let get_diary_deps = GetDiaryDeps {
        diary: deps.diary.clone(),
    };
    let page = get_diary::execute(&get_diary_deps, query).await?;

    let display_name = user
        .email()
        .value()
        .split('@')
        .next()
        .unwrap_or("User")
        .to_string();

    Ok(UserFeed {
        author: FeedAuthor { display_name },
        entries: page.items,
    })
}
