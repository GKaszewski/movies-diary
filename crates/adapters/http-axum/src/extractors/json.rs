use axum::{
    extract::{
        FromRequest, FromRequestParts, Multipart, Path, Query, Request,
        multipart::MultipartRejection,
        rejection::{JsonRejection, PathRejection, QueryRejection},
    },
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use serde::de::DeserializeOwned;

use crate::errors::{INTERNAL, INVALID_INPUT, refuse};

pub struct MalformedRequest {
    status: StatusCode,
    message: String,
}

impl IntoResponse for MalformedRequest {
    fn into_response(self) -> Response {
        let code = if self.status.is_server_error() {
            INTERNAL
        } else {
            INVALID_INPUT
        };
        refuse(self.status, code, self.message)
    }
}

impl From<JsonRejection> for MalformedRequest {
    fn from(r: JsonRejection) -> Self {
        Self {
            status: r.status(),
            message: r.body_text(),
        }
    }
}

impl From<QueryRejection> for MalformedRequest {
    fn from(r: QueryRejection) -> Self {
        Self {
            status: r.status(),
            message: r.body_text(),
        }
    }
}

impl From<PathRejection> for MalformedRequest {
    fn from(r: PathRejection) -> Self {
        Self {
            status: r.status(),
            message: r.body_text(),
        }
    }
}

impl From<MultipartRejection> for MalformedRequest {
    fn from(r: MultipartRejection) -> Self {
        Self {
            status: r.status(),
            message: r.body_text(),
        }
    }
}

pub struct Upload(pub Multipart);

impl<S> FromRequest<S> for Upload
where
    S: Send + Sync,
{
    type Rejection = MalformedRequest;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(Multipart::from_request(req, state).await?))
    }
}

pub struct Body<T>(pub T);

impl<S, T> FromRequest<S> for Body<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = MalformedRequest;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let axum::Json(v) = axum::Json::<T>::from_request(req, state).await?;
        Ok(Self(v))
    }
}

pub struct Params<T>(pub T);

impl<S, T> FromRequestParts<S> for Params<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = MalformedRequest;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(v) = Query::<T>::from_request_parts(parts, state).await?;
        Ok(Self(v))
    }
}

pub struct PathId<T>(pub T);

impl<S, T> FromRequestParts<S> for PathId<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Send,
{
    type Rejection = MalformedRequest;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(v) = Path::<T>::from_request_parts(parts, state).await?;
        Ok(Self(v))
    }
}
