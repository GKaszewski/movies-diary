use poster_fetcher::{PosterFetcherConfig, ReqwestPosterFetcher};

#[test]
fn poster_fetcher_builds_without_bootstrap() {
    ReqwestPosterFetcher::new(PosterFetcherConfig { timeout_seconds: 5 }).expect("client builds");
}
