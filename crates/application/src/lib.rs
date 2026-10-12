pub mod config;
pub mod deps;
pub mod jobs;
pub mod ports;
pub mod services;
pub mod worker;

pub mod auth;
pub mod diary;
pub mod goals;
pub mod import;
pub mod insights;
pub mod integrations;
pub mod movies;
pub mod person;
pub mod search;
pub mod social;
pub mod users;
pub mod watchlist;
pub mod wrapup;

#[cfg(test)]
pub mod test_helpers;

#[cfg(test)]
#[path = "tests/services.rs"]
mod services_tests;

pub use deps::Deps;
pub use deps::{WorkerDeps, WorkerServices};
pub use movies::MovieDiscoveryIndexer;
pub use movies::SearchCleanupHandler;
pub use movies::SearchReindexHandler;
pub use services::Services;
