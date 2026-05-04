use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{Movie, Review},
    ports::MetadataSearchCriteria,
    value_objects::{Comment, ExternalMetadataId, MovieTitle, Rating, ReleaseYear, UserId},
};

use crate::{commands::LogReviewCommand, context::AppContext};

pub async fn execute(ctx: &AppContext, cmd: LogReviewCommand) -> Result<(), DomainError> {
    let rating = Rating::new(cmd.rating)?;
    let user_id = UserId::from_uuid(cmd.user_id);
    let comment = cmd.comment.clone().map(Comment::new).transpose()?;

    let (movie, is_new_movie) = resolve_movie(ctx, &cmd).await?;

    ctx.repository.upsert_movie(&movie).await?;

    let review = Review::new(movie.id().clone(), user_id, rating, comment, cmd.watched_at)?;
    let review_event = ctx.repository.save_review(&review).await?;

    publish_events(ctx, &movie, is_new_movie, review_event).await?;

    Ok(())
}

async fn resolve_movie(
    ctx: &AppContext,
    cmd: &LogReviewCommand,
) -> Result<(Movie, bool), DomainError> {
    if let Some(ext_id_str) = &cmd.external_metadata_id {
        if let Some(resolved) = resolve_external_movie(ctx, ext_id_str).await? {
            return Ok(resolved);
        }
    }

    if let Some(title) = &cmd.manual_title {
        if let Some(resolved) = resolve_by_title(ctx, title, cmd.manual_release_year).await? {
            return Ok(resolved);
        }
    }

    resolve_manual_movie(ctx, cmd).await
}

async fn resolve_external_movie(
    ctx: &AppContext,
    ext_id_str: &str,
) -> Result<Option<(Movie, bool)>, DomainError> {
    let tmdb_id = ExternalMetadataId::new(ext_id_str.to_string())?;

    if let Some(m) = ctx.repository.get_movie_by_external_id(&tmdb_id).await? {
        return Ok(Some((m, false)));
    }

    match ctx
        .metadata_client
        .fetch_movie_metadata(&MetadataSearchCriteria::ImdbId(tmdb_id))
        .await
    {
        Ok(m) => Ok(Some((m, true))),
        Err(e) => {
            tracing::warn!(
                "Failed to fetch from TMDB, falling back to manual entry: {:?}",
                e
            );
            Ok(None)
        }
    }
}

async fn resolve_by_title(
    ctx: &AppContext,
    title: &str,
    year: Option<u16>,
) -> Result<Option<(Movie, bool)>, DomainError> {
    let criteria = MetadataSearchCriteria::Title { title: title.to_string(), year };
    match ctx.metadata_client.fetch_movie_metadata(&criteria).await {
        Ok(m) => Ok(Some((m, true))),
        Err(e) => {
            tracing::warn!("OMDb title search failed, falling back to manual: {:?}", e);
            Ok(None)
        }
    }
}

async fn resolve_manual_movie(
    ctx: &AppContext,
    cmd: &LogReviewCommand,
) -> Result<(Movie, bool), DomainError> {
    let title_str = cmd.manual_title.as_ref().ok_or_else(|| {
        DomainError::ValidationError(
            "Manual title required if TMDB fetch fails or is omitted".into(),
        )
    })?;
    let year_val = cmd.manual_release_year.ok_or_else(|| {
        DomainError::ValidationError(
            "Manual release year required if TMDB fetch fails or is omitted".into(),
        )
    })?;

    let title = MovieTitle::new(title_str.clone())?;
    let release_year = ReleaseYear::new(year_val)?;

    let candidates = ctx
        .repository
        .get_movies_by_title_and_year(&title, &release_year)
        .await?;

    let matched_movie = candidates
        .into_iter()
        .find(|m| m.is_manual_match(&title, &release_year, cmd.manual_director.as_deref()));

    if let Some(existing_movie) = matched_movie {
        Ok((existing_movie, false))
    } else {
        let new_movie = Movie::new(None, title, release_year, cmd.manual_director.clone(), None);
        Ok((new_movie, true))
    }
}

async fn publish_events(
    ctx: &AppContext,
    movie: &Movie,
    is_new_movie: bool,
    review_event: DomainEvent,
) -> Result<(), DomainError> {
    if is_new_movie {
        if let Some(ext_id) = movie.external_metadata_id() {
            let discovery_event = DomainEvent::MovieDiscovered {
                movie_id: movie.id().clone(),
                external_metadata_id: ext_id.clone(),
            };
            ctx.event_publisher.publish(&discovery_event).await?;
        }
    }

    ctx.event_publisher.publish(&review_event).await?;
    Ok(())
}
