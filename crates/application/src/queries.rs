use domain::models::{ExportFormat, SortDirection};
use uuid::Uuid;

pub struct LoginQuery {
    pub email: String,
    pub password: String,
}

pub struct ExportQuery {
    pub user_id: Uuid,
    pub format: ExportFormat,
}

pub struct GetDiaryQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub sort_by: Option<SortDirection>,
    pub movie_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
}

pub struct GetReviewHistoryQuery {
    pub movie_id: Uuid,
}

pub struct GetActivityFeedQuery {
    pub limit: u32,
    pub offset: u32,
    pub sort_by: domain::ports::FeedSortBy,
    pub search: Option<String>,
    pub viewer_user_id: Option<Uuid>,
    pub filter_following: bool,
}

pub struct GetUsersQuery;

#[derive(Debug, Clone, Copy, Default)]
pub enum ProfileView {
    History,
    Trends,
    Ratings,
    #[default]
    Recent,
}

impl ProfileView {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::History => "history",
            Self::Trends => "trends",
            Self::Ratings => "ratings",
            Self::Recent => "recent",
        }
    }
}

impl std::str::FromStr for ProfileView {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "history" => Ok(Self::History),
            "trends" => Ok(Self::Trends),
            "ratings" => Ok(Self::Ratings),
            "recent" => Ok(Self::Recent),
            other => Err(format!("unknown profile view: {other}")),
        }
    }
}

pub struct GetUserProfileQuery {
    pub user_id: Uuid,
    pub view: ProfileView,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub sort_by: domain::ports::FeedSortBy,
    pub search: Option<String>,
    pub is_own_profile: bool,
}

pub struct GetMovieSocialPageQuery {
    pub movie_id: uuid::Uuid,
    pub limit: u32,
    pub offset: u32,
}

pub struct GetMoviesQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub search: Option<String>,
    pub genre: Option<String>,
    pub language: Option<String>,
}

pub struct GetWatchlistQuery {
    pub user_id: Uuid,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

pub struct IsOnWatchlistQuery {
    pub user_id: Uuid,
    pub movie_id: Uuid,
}

pub struct GetCurrentProfileQuery {
    pub user_id: Uuid,
}

pub struct GetWatchQueueQuery {
    pub user_id: Uuid,
}

pub struct GetWebhookTokensQuery {
    pub user_id: Uuid,
}
