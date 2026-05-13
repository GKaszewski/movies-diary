use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::WatchlistEntry,
    value_objects::{MovieId, UserId},
};

use crate::{
    commands::AddToWatchlistCommand,
    context::AppContext,
    movie_resolver::{MovieResolver, MovieResolverDeps},
};

pub async fn execute(ctx: &AppContext, cmd: AddToWatchlistCommand) -> Result<(), DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);

    let movie = if let Some(id) = cmd.input.movie_id {
        let movie_id = MovieId::from_uuid(id);
        ctx.movie_repository
            .get_movie_by_id(&movie_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Movie {id}")))?
    } else {
        let deps = MovieResolverDeps {
            repository: ctx.movie_repository.as_ref(),
            metadata_client: ctx.metadata_client.as_ref(),
        };
        let (movie, is_new) = MovieResolver::default_pipeline()
            .resolve(&cmd.input, &deps)
            .await?;
        if is_new {
            ctx.movie_repository.upsert_movie(&movie).await?;
            if let Some(ext_id) = movie.external_metadata_id() {
                let _ = ctx
                    .event_publisher
                    .publish(&DomainEvent::MovieDiscovered {
                        movie_id: movie.id().clone(),
                        external_metadata_id: ext_id.clone(),
                    })
                    .await;
            }
        }
        movie
    };

    let entry = WatchlistEntry::new(user_id.clone(), movie.id().clone());
    ctx.watchlist_repository.add(&entry).await?;

    let _ = ctx
        .event_publisher
        .publish(&DomainEvent::WatchlistEntryAdded {
            user_id,
            movie_id: movie.id().clone(),
            movie_title: movie.title().value().to_string(),
            release_year: movie.release_year().value(),
            external_metadata_id: movie.external_metadata_id().map(|e| e.value().to_string()),
            added_at: entry.added_at,
        })
        .await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use domain::{
        models::Movie,
        ports::MovieRepository,
        value_objects::{MovieTitle, ReleaseYear},
        testing::{InMemoryMovieRepository, InMemoryWatchlistRepository},
    };

    use crate::{
        commands::{AddToWatchlistCommand, MovieInput},
        test_helpers::TestContextBuilder,
        use_cases::add_to_watchlist,
    };

    #[tokio::test]
    async fn test_add_to_watchlist_resolves_and_saves() {
        let movies = InMemoryMovieRepository::new();
        let watchlist = InMemoryWatchlistRepository::new();

        let movie = Movie::new(
            None,
            MovieTitle::new("The Thing".into()).unwrap(),
            ReleaseYear::new(1982).unwrap(),
            None,
            None,
        );
        let movie_uuid = movie.id().value();
        movies.upsert_movie(&movie).await.unwrap();

        let ctx = TestContextBuilder::new()
            .with_movies(Arc::clone(&movies) as _)
            .with_watchlist(Arc::clone(&watchlist) as _)
            .build();

        let cmd = AddToWatchlistCommand {
            user_id: uuid::Uuid::new_v4(),
            input: MovieInput {
                movie_id: Some(movie_uuid),
                external_metadata_id: None,
                manual_title: None,
                manual_release_year: None,
                manual_director: None,
            },
        };

        add_to_watchlist::execute(&ctx, cmd).await.unwrap();

        assert_eq!(watchlist.count(), 1);
    }

    #[tokio::test]
    async fn test_add_to_watchlist_already_present_is_idempotent() {
        let movies = InMemoryMovieRepository::new();
        let watchlist = InMemoryWatchlistRepository::new();

        let movie = Movie::new(
            None,
            MovieTitle::new("RoboCop".into()).unwrap(),
            ReleaseYear::new(1987).unwrap(),
            None,
            None,
        );
        let movie_uuid = movie.id().value();
        let user_id = uuid::Uuid::new_v4();
        movies.upsert_movie(&movie).await.unwrap();

        let ctx = TestContextBuilder::new()
            .with_movies(Arc::clone(&movies) as _)
            .with_watchlist(Arc::clone(&watchlist) as _)
            .build();

        let make_cmd = || AddToWatchlistCommand {
            user_id,
            input: MovieInput {
                movie_id: Some(movie_uuid),
                external_metadata_id: None,
                manual_title: None,
                manual_release_year: None,
                manual_director: None,
            },
        };

        add_to_watchlist::execute(&ctx, make_cmd()).await.unwrap();
        add_to_watchlist::execute(&ctx, make_cmd()).await.unwrap();

        assert_eq!(watchlist.count(), 1, "idempotent add should not duplicate");
    }
}
