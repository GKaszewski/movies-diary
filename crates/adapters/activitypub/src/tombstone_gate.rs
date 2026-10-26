use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use domain::{ports::UserRepository, value_objects::UserId};

pub async fn tombstone_gate(
    State(users): State<Arc<dyn UserRepository>>,
    request: Request,
    next: Next,
) -> Response {
    if let Some(id) = gated_user(request.uri().path())
        && matches!(
            users.find_tombstoned_by_id(&UserId::from_uuid(id)).await,
            Ok(Some(_))
        )
    {
        return StatusCode::GONE.into_response();
    }
    next.run(request).await
}

pub fn gated_user(path: &str) -> Option<uuid::Uuid> {
    let rest = path.strip_prefix("/users/")?;
    let (id, endpoint) = rest.split_once('/')?;
    (!endpoint.is_empty()).then_some(())?;
    uuid::Uuid::parse_str(id).ok()
}
