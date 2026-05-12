use std::sync::Arc;

use anyhow::Context;
use domain::ports::{
    DiaryRepository, ImageRefCommand, ImageRefQuery, ImportProfileRepository,
    ImportSessionRepository, MovieProfileRepository, MovieRepository, ReviewRepository,
    StatsRepository, UserRepository,
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
    pub image_ref_command: Arc<dyn ImageRefCommand>,
    pub image_ref_query: Arc<dyn ImageRefQuery>,
}

pub async fn connect(database_url: &str, backend: &str) -> anyhow::Result<(Repos, DbPool)> {
    match backend {
        #[cfg(feature = "postgres")]
        "postgres" => {
            let (pool, m, r, d, s, u, is, ip, mp) =
                postgres::wire(database_url).await.context("PostgreSQL connection failed")?;
            let (image_ref_command, image_ref_query) = postgres::create_image_ref(pool.clone());
            Ok((Repos { movie: m, review: r, diary: d, stats: s, user: u,
                        import_session: is, import_profile: ip, movie_profile: mp,
                        image_ref_command, image_ref_query }, DbPool::Postgres(pool)))
        }
        #[cfg(feature = "sqlite")]
        _ => {
            let (pool, m, r, d, s, u, is, ip, mp) =
                sqlite::wire(database_url).await.context("SQLite connection failed")?;
            let (image_ref_command, image_ref_query) = sqlite::create_image_ref(pool.clone());
            Ok((Repos { movie: m, review: r, diary: d, stats: s, user: u,
                        import_session: is, import_profile: ip, movie_profile: mp,
                        image_ref_command, image_ref_query }, DbPool::Sqlite(pool)))
        }
        #[cfg(not(feature = "sqlite"))]
        _ => anyhow::bail!("DATABASE_BACKEND={backend} is not supported by this build"),
    }
}
