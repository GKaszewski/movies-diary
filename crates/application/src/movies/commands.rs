pub struct EnrichMovieCommand {
    pub movie_id: domain::value_objects::MovieId,
    pub profile: domain::models::MovieProfile,
}
