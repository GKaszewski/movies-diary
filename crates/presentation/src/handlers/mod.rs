pub mod auth;
pub mod diary;
pub mod goals;
mod helpers;
pub mod images;
pub mod import;
pub mod integrations;
pub mod movies;
pub mod rss;
pub mod search;
#[cfg(feature = "federation")]
pub mod social;
pub mod users;
pub mod watchlist;
pub mod webhook;
pub mod wrapup;

const DEFAULT_PAGE_LIMIT: u32 = 5;
const RSS_FEED_LIMIT: u32 = 50;
