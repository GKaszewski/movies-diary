use std::sync::Arc;

use anyhow::Context;
use domain::ports::{
    DiaryRepository, ImageRefCommand, ImageRefQuery, ImportProfileRepository,
    ImportSessionRepository, MovieProfileRepository, MovieRepository, PersonCommand, PersonQuery,
    ReviewRepository, SearchCommand, SearchPort, StatsRepository, UserRepository, WatchlistRepository,
};

pub enum DbPool {
    #[cfg(feature = "sqlite")]
    Sqlite(sqlx::SqlitePool),
    #[cfg(feature = "postgres")]
    Postgres(sqlx::PgPool),
}

pub struct Repos {
    pub movie: Arc<dyn MovieRepository>,
    pub review: Arc<dyn ReviewRepository>,
    pub diary: Arc<dyn DiaryRepository>,
    pub stats: Arc<dyn StatsRepository>,
    pub user: Arc<dyn UserRepository>,
    pub import_session: Arc<dyn ImportSessionRepository>,
    pub import_profile: Arc<dyn ImportProfileRepository>,
    pub movie_profile: Arc<dyn MovieProfileRepository>,
    pub watchlist: Arc<dyn WatchlistRepository>,
    pub image_ref_command: Arc<dyn ImageRefCommand>,
    pub image_ref_query:   Arc<dyn ImageRefQuery>,
    pub person_command:    Arc<dyn PersonCommand>,
    pub person_query:      Arc<dyn PersonQuery>,
    pub search_command:    Arc<dyn SearchCommand>,
    pub search_port:       Arc<dyn SearchPort>,
}

pub async fn connect(database_url: &str, backend: &str) -> anyhow::Result<(Repos, DbPool)> {
    match backend {
        #[cfg(feature = "postgres")]
        "postgres" => {
            let (pool, m, r, d, s, u, is, ip, mp, wl) =
                postgres::wire(database_url).await.context("PostgreSQL connection failed")?;
            let (image_ref_command, image_ref_query) = postgres::create_image_ref(pool.clone());
            let (person_command, person_query) = postgres::create_person_adapter(pool.clone());
            let (search_command, search_port)  = postgres_search::create_search_adapter(pool.clone());
            Ok((Repos { movie: m, review: r, diary: d, stats: s, user: u,
                        import_session: is, import_profile: ip, movie_profile: mp, watchlist: wl,
                        image_ref_command, image_ref_query,
                        person_command, person_query, search_command, search_port },
                DbPool::Postgres(pool)))
        }
        #[cfg(feature = "sqlite")]
        _ => {
            let (pool, m, r, d, s, u, is, ip, mp, wl) =
                sqlite::wire(database_url).await.context("SQLite connection failed")?;
            let (image_ref_command, image_ref_query) = sqlite::create_image_ref(pool.clone());
            let (person_command, person_query) = sqlite::create_person_adapter(pool.clone());
            let (search_command, search_port)  = sqlite_search::create_search_adapter(pool.clone());
            Ok((Repos { movie: m, review: r, diary: d, stats: s, user: u,
                        import_session: is, import_profile: ip, movie_profile: mp, watchlist: wl,
                        image_ref_command, image_ref_query,
                        person_command, person_query, search_command, search_port },
                DbPool::Sqlite(pool)))
        }
        #[cfg(not(feature = "sqlite"))]
        _ => anyhow::bail!("DATABASE_BACKEND={backend} is not supported by this build"),
    }
}
