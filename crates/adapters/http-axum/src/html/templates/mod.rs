mod auth;
mod common;
mod diary;
mod import;
mod integrations;
mod movies;
mod social;
mod users;
mod watchlist;
mod wrapup;

pub use auth::*;
pub use common::*;
pub use diary::*;
pub use import::*;
pub use integrations::*;
pub use movies::*;
pub use social::*;
pub use users::*;
pub use watchlist::*;
pub use wrapup::*;

pub(super) mod filters {
    #[askama::filter_fn]
    pub fn poster_src<T: std::fmt::Display>(
        path: T,
        _env: &dyn askama::Values,
    ) -> askama::Result<String> {
        let p = path.to_string();
        if p.starts_with("http://") || p.starts_with("https://") {
            Ok(p)
        } else {
            Ok(format!("/images/{}", p))
        }
    }
}
