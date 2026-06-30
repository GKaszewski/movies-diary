use api_types::FeedEntryDto;
use domain::models::FeedEntry;

use super::movies::{movie_to_dto, review_to_dto};

pub fn feed_entry_to_dto(e: &FeedEntry) -> FeedEntryDto {
    use domain::models::ReviewSource;
    let actor_url = match e.review().source() {
        ReviewSource::Remote { actor_url } => Some(actor_url.clone()),
        ReviewSource::Local => None,
    };
    FeedEntryDto {
        movie: movie_to_dto(e.movie()),
        review: review_to_dto(e.review()),
        user_id: e.review().user_id().value(),
        user_display_name: e.user_display_name().to_string(),
        is_federated: e.review().is_remote(),
        actor_url,
    }
}
