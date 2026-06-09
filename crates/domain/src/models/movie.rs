use crate::value_objects::{ExternalMetadataId, MovieId, MovieTitle, PosterPath, ReleaseYear};

#[derive(Clone, Debug, Default)]
pub struct MovieFilter {
    pub search: Option<String>,
    pub genre: Option<String>,
    pub language: Option<String>,
}

#[derive(Clone, Debug)]
pub struct MovieSummary {
    pub movie: Movie,
    pub genres: Vec<String>,
    pub runtime_minutes: Option<u32>,
    pub original_language: Option<String>,
    pub overview: Option<String>,
    pub collection_name: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Movie {
    id: MovieId,
    external_metadata_id: Option<ExternalMetadataId>,
    title: MovieTitle,
    release_year: ReleaseYear,
    director: Option<String>,
    poster_path: Option<PosterPath>,
}

impl Movie {
    pub fn new(
        external_metadata_id: Option<ExternalMetadataId>,
        title: MovieTitle,
        release_year: ReleaseYear,
        director: Option<String>,
        poster_path: Option<PosterPath>,
    ) -> Self {
        Self {
            id: MovieId::generate(),
            external_metadata_id,
            title,
            release_year,
            director,
            poster_path,
        }
    }

    pub fn from_persistence(
        id: MovieId,
        external_metadata_id: Option<ExternalMetadataId>,
        title: MovieTitle,
        release_year: ReleaseYear,
        director: Option<String>,
        poster_path: Option<PosterPath>,
    ) -> Self {
        Self {
            id,
            external_metadata_id,
            title,
            release_year,
            director,
            poster_path,
        }
    }

    pub fn update_poster(&mut self, poster_path: PosterPath) {
        self.poster_path = Some(poster_path);
    }

    pub fn id(&self) -> &MovieId {
        &self.id
    }
    pub fn external_metadata_id(&self) -> Option<&ExternalMetadataId> {
        self.external_metadata_id.as_ref()
    }
    pub fn title(&self) -> &MovieTitle {
        &self.title
    }
    pub fn release_year(&self) -> &ReleaseYear {
        &self.release_year
    }
    pub fn director(&self) -> Option<&str> {
        self.director.as_deref()
    }
    pub fn poster_path(&self) -> Option<&PosterPath> {
        self.poster_path.as_ref()
    }
}

impl Movie {
    pub fn is_manual_match(
        &self,
        title: &MovieTitle,
        year: &ReleaseYear,
        director: Option<&str>,
    ) -> bool {
        if self.title != *title || self.release_year != *year {
            return false;
        }

        match (self.director(), director) {
            (Some(existing_dir), Some(new_dir)) => existing_dir.eq_ignore_ascii_case(new_dir),
            _ => true,
        }
    }
}
