use uuid::Uuid;

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

pub struct GetCurrentProfileQuery {
    pub user_id: Uuid,
}
