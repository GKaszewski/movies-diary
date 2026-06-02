use uuid::Uuid;

use crate::diary::commands::MovieInput;

pub struct AddToWatchlistCommand {
    pub user_id: Uuid,
    pub input: MovieInput,
}

pub struct RemoveFromWatchlistCommand {
    pub user_id: Uuid,
    pub movie_id: Uuid,
}
