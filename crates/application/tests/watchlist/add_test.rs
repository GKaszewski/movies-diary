use crate::support::{add_to_watchlist_command, movie_input_manual};
use domain::testing::test_movie_with_year;
use std::sync::Arc;

use domain::{
    events::DomainEvent,
    ports::MovieCommand,
    testing::{InMemoryStore, RefusingWatchlistRepository},
};

use application::watchlist::{add, commands::AddToWatchlistCommand, deps::WatchlistAddDeps};

fn make_deps(movies: Arc<InMemoryStore>, watchlist: Arc<InMemoryStore>) -> WatchlistAddDeps {
    WatchlistAddDeps {
        movie_command: Arc::clone(&movies) as _,
        movie_query: movies,
        metadata: Arc::new(domain::testing::FakeMetadataClient),
        watchlist,
    }
}

#[tokio::test]
async fn adding_to_the_watchlist_resolves_the_movie_and_saves_the_entry() {
    let movies = InMemoryStore::new();
    let watchlist = Arc::clone(&movies);

    let movie = test_movie_with_year("The Thing", 1982);
    let movie_uuid = movie.id().value();
    movies.upsert_movie(&movie, &[]).await.unwrap();

    let deps = make_deps(Arc::clone(&movies), Arc::clone(&watchlist));

    let cmd = add_to_watchlist_command(uuid::Uuid::new_v4(), movie_uuid);

    add::execute(&deps, cmd).await.unwrap();

    assert_eq!(watchlist.watchlist_count(), 1);
}

#[tokio::test]
async fn adding_a_movie_already_on_the_watchlist_is_idempotent() {
    let movies = InMemoryStore::new();
    let watchlist = Arc::clone(&movies);

    let movie = test_movie_with_year("RoboCop", 1987);
    let movie_uuid = movie.id().value();
    let user_id = uuid::Uuid::new_v4();
    movies.upsert_movie(&movie, &[]).await.unwrap();

    let deps = make_deps(Arc::clone(&movies), Arc::clone(&watchlist));

    let make_cmd = || add_to_watchlist_command(user_id, movie_uuid);

    add::execute(&deps, make_cmd()).await.unwrap();
    add::execute(&deps, make_cmd()).await.unwrap();

    assert_eq!(
        movies.recorded_events().len(),
        1,
        "the repeat inserted nothing, so it announces nothing"
    );
    assert_eq!(
        watchlist.watchlist_count(),
        1,
        "idempotent add should not duplicate"
    );
}

#[tokio::test]
async fn adding_a_manual_movie_to_the_watchlist_creates_and_saves_it() {
    let movies = InMemoryStore::new();
    let watchlist = Arc::clone(&movies);

    let deps = make_deps(Arc::clone(&movies), Arc::clone(&watchlist));

    let cmd = AddToWatchlistCommand {
        user_id: uuid::Uuid::new_v4(),
        input: movie_input_manual("New Manual Movie", 2024),
    };

    add::execute(&deps, cmd).await.unwrap();

    assert_eq!(watchlist.watchlist_count(), 1);
    assert_eq!(movies.movie_count(), 1);
}

#[tokio::test]
async fn adding_an_unknown_movie_id_to_the_watchlist_fails() {
    let movies = InMemoryStore::new();
    let watchlist = Arc::clone(&movies);

    let deps = make_deps(Arc::clone(&movies), Arc::clone(&watchlist));

    let cmd = add_to_watchlist_command(uuid::Uuid::new_v4(), uuid::Uuid::new_v4());

    assert!(add::execute(&deps, cmd).await.is_err());
}

#[tokio::test]
async fn adding_the_same_movie_to_a_watchlist_twice_keeps_a_single_entry_and_succeeds() {
    let movies = InMemoryStore::new();
    let movie = test_movie_with_year("Heat", 1995);
    let movie_uuid = movie.id().value();
    let user_id = uuid::Uuid::new_v4();
    movies.upsert_movie(&movie, &[]).await.unwrap();
    let deps = make_deps(Arc::clone(&movies), Arc::clone(&movies));

    for _ in 0..2 {
        add::execute(&deps, add_to_watchlist_command(user_id, movie_uuid))
            .await
            .expect("second add must not error");
    }

    assert_eq!(movies.watchlist_count(), 1);
}

#[tokio::test]
async fn adding_an_entry_records_the_entry_and_its_event_together() {
    let movies = InMemoryStore::new();
    let movie = test_movie_with_year("Heat", 1995);
    movies.upsert_movie(&movie, &[]).await.unwrap();
    let deps = make_deps(Arc::clone(&movies), Arc::clone(&movies));

    add::execute(
        &deps,
        add_to_watchlist_command(uuid::Uuid::new_v4(), movie.id().value()),
    )
    .await
    .unwrap();

    assert_eq!(movies.watchlist_count(), 1);
    let recorded = movies.recorded_events();
    assert_eq!(recorded.len(), 1);
    assert!(matches!(
        recorded[0],
        DomainEvent::WatchlistEntryAdded { .. }
    ));
}

#[tokio::test]
async fn a_refused_add_propagates_and_records_nothing() {
    let movies = InMemoryStore::new();
    let movie = test_movie_with_year("Heat", 1995);
    movies.upsert_movie(&movie, &[]).await.unwrap();
    let deps = WatchlistAddDeps {
        watchlist: Arc::new(RefusingWatchlistRepository),
        ..make_deps(Arc::clone(&movies), Arc::clone(&movies))
    };

    let result = add::execute(
        &deps,
        add_to_watchlist_command(uuid::Uuid::new_v4(), movie.id().value()),
    )
    .await;

    assert!(matches!(
        result,
        Err(domain::errors::DomainError::InfrastructureError(m)) if m == "refused"
    ));
    assert_eq!(movies.watchlist_count(), 0);
    assert!(movies.recorded_events().is_empty());
}
