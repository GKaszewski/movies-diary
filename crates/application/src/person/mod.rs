pub mod deps;
pub mod enrich;
pub mod event_handler;
pub mod get;
pub mod get_credits;

pub use event_handler::PersonEnrichmentHandler;

use chrono::Utc;
use domain::models::Person;

pub(crate) const ENRICHMENT_TTL_DAYS: i64 = 90;

pub(crate) fn should_enrich(p: &Person) -> bool {
    match p.enriched_at() {
        None => true,
        Some(at) => (Utc::now() - at).num_days() >= ENRICHMENT_TTL_DAYS,
    }
}
