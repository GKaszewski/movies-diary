pub struct GetMoviesQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub search: Option<String>,
    pub genre: Option<String>,
    pub language: Option<String>,
}
