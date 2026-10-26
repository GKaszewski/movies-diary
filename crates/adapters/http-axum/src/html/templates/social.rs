use api_types::HtmlPageContext;
use askama::Template;

pub struct RemoteActorData {
    pub handle: String,
    pub display_name: Option<String>,
    pub url: String,
    pub avatar_url: Option<String>,
}

#[derive(Template)]
#[template(path = "following.html")]
pub struct FollowingTemplate {
    pub ctx: HtmlPageContext,
    pub user_id: uuid::Uuid,
    pub actors: Vec<RemoteActorData>,
    pub pending_actors: Vec<RemoteActorData>,
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "followers.html")]
pub struct FollowersTemplate {
    pub ctx: HtmlPageContext,
    pub user_id: uuid::Uuid,
    pub actors: Vec<RemoteActorData>,
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "blocked_domains.html")]
pub struct BlockedDomainsTemplate<'a> {
    pub ctx: &'a HtmlPageContext,
    pub domains: &'a [BlockedDomainEntry],
}

#[derive(Template)]
#[template(path = "blocked_actors.html")]
pub struct BlockedActorsTemplate<'a> {
    pub ctx: &'a HtmlPageContext,
    pub actors: &'a [BlockedActorEntry],
}

pub struct BlockedDomainEntry {
    pub domain: String,
    pub reason: Option<String>,
    pub blocked_at: String,
}

pub struct BlockedActorEntry {
    pub url: String,
    pub handle: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}
