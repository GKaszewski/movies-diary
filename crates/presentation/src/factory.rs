use std::sync::Arc;

use anyhow::Context;

use domain::ports::{
    AuthService, DiaryRepository, ImageStorage, ImportProfileRepository,
    ImportSessionRepository, LocalApContentQuery, MetadataClient, MovieProfileRepository,
    MovieRepository, PasswordHasher, PersonCommand, PersonQuery, PosterFetcherClient,
    ReviewRepository, SearchCommand, SearchPort, StatsRepository, UserProfileFieldsRepository,
    UserRepository, WatchlistRepository,
};

pub struct DatabaseAdapters {
    pub movie_repo: Arc<dyn MovieRepository>,
    pub review_repo: Arc<dyn ReviewRepository>,
    pub diary_repo: Arc<dyn DiaryRepository>,
    pub stats_repo: Arc<dyn StatsRepository>,
    pub user_repo: Arc<dyn UserRepository>,
    pub import_session_repo: Arc<dyn ImportSessionRepository>,
    pub import_profile_repo: Arc<dyn ImportProfileRepository>,
    pub movie_profile_repo: Arc<dyn MovieProfileRepository>,
    pub watchlist_repo: Arc<dyn WatchlistRepository>,
    pub ap_content_repo: Arc<dyn LocalApContentQuery>,
    pub person_command: Arc<dyn PersonCommand>,
    pub person_query: Arc<dyn PersonQuery>,
    pub search_port: Arc<dyn SearchPort>,
    pub search_command: Arc<dyn SearchCommand>,
    pub profile_fields_repo: Arc<dyn UserProfileFieldsRepository>,
    pub db_pool: DbPool,
}

pub enum DbPool {
    #[cfg(feature = "sqlite")]
    Sqlite(sqlx::SqlitePool),
    #[cfg(feature = "postgres")]
    Postgres(sqlx::PgPool),
}

pub async fn build_database_adapters(
    backend: &str,
    url: &str,
) -> anyhow::Result<DatabaseAdapters> {
    match backend {
        #[cfg(feature = "postgres")]
        "postgres" => {
            let (pool, m, r, d, s, u, is, ip, mp, wl, ac) = postgres::wire(url)
                .await
                .context("PostgreSQL connection failed")?;
            let (pc, pq) = postgres::create_person_adapter(pool.clone());
            let (sc, sp) = postgres_search::create_search_adapter(pool.clone());
            let pf = postgres::create_profile_fields_repo(pool.clone());
            Ok(DatabaseAdapters {
                movie_repo: m,
                review_repo: r,
                diary_repo: d,
                stats_repo: s,
                user_repo: u,
                import_session_repo: is,
                import_profile_repo: ip,
                movie_profile_repo: mp,
                watchlist_repo: wl,
                ap_content_repo: ac,
                person_command: pc,
                person_query: pq,
                search_port: sp,
                search_command: sc,
                profile_fields_repo: pf,
                db_pool: DbPool::Postgres(pool),
            })
        }
        #[cfg(feature = "sqlite")]
        _ => {
            let (pool, m, r, d, s, u, is, ip, mp, wl, ac) = sqlite::wire(url)
                .await
                .context("SQLite connection failed")?;
            let (pc, pq) = sqlite::create_person_adapter(pool.clone());
            let (sc, sp) = sqlite_search::create_search_adapter(pool.clone());
            let pf = sqlite::create_profile_fields_repo(pool.clone());
            Ok(DatabaseAdapters {
                movie_repo: m,
                review_repo: r,
                diary_repo: d,
                stats_repo: s,
                user_repo: u,
                import_session_repo: is,
                import_profile_repo: ip,
                movie_profile_repo: mp,
                watchlist_repo: wl,
                ap_content_repo: ac,
                person_command: pc,
                person_query: pq,
                search_port: sp,
                search_command: sc,
                profile_fields_repo: pf,
                db_pool: DbPool::Sqlite(pool),
            })
        }
        #[cfg(not(feature = "sqlite"))]
        _ => anyhow::bail!(
            "DATABASE_BACKEND={backend} is not supported by this build (enable sqlite or postgres feature)"
        ),
    }
}

pub fn build_auth_adapters() -> anyhow::Result<(Arc<dyn AuthService>, Arc<dyn PasswordHasher>)> {
    auth::create()
}

pub fn build_metadata_client() -> anyhow::Result<Arc<dyn MetadataClient>> {
    metadata::create()
}

pub fn build_poster_fetcher() -> anyhow::Result<Arc<dyn PosterFetcherClient>> {
    poster_fetcher::create()
}

pub fn build_image_storage() -> anyhow::Result<Arc<dyn ImageStorage>> {
    image_storage::create()
}

pub fn build_profile_fields_repo(pool: &DbPool) -> anyhow::Result<Arc<dyn UserProfileFieldsRepository>> {
    match pool {
        #[cfg(feature = "postgres")]
        DbPool::Postgres(pool) => Ok(postgres::create_profile_fields_repo(pool.clone())),
        #[cfg(feature = "sqlite")]
        DbPool::Sqlite(pool) => Ok(sqlite::create_profile_fields_repo(pool.clone())),
        #[cfg(not(feature = "sqlite"))]
        _ => anyhow::bail!("no profile fields repo for this backend"),
    }
}
