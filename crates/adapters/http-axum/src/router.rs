use std::num::NonZeroU32;

use axum::{Router, http::StatusCode, routing};
use axum_governor::{
    GovernorConfigBuilder, GovernorLayer, Quota, RejectionReason, extractor::PeerIp,
};
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

use crate::{
    api,
    errors::{INTERNAL, RATE_LIMITED, refuse},
    federation, html,
    state::AppState,
};

pub fn build_router(state: AppState, ap_router: Router) -> Router {
    let rate_limit = state.app_ctx.config.rate_limit;
    let secure_cookies = state.app_ctx.config.secure_cookies;
    let cors_origins = state.app_ctx.config.cors_origins.clone();

    let ap_cfg = GovernorConfigBuilder::default()
        .with_extractor(PeerIp::default())
        .expect_connect_info()
        .quota_default(per_minute(rate_limit / 2))
        .finish()
        .unwrap();
    let ap_router = ap_router.layer(GovernorLayer::new(ap_cfg));

    Router::new()
        .route("/health", routing::get(health_handler))
        .merge(html_routes(rate_limit, secure_cookies))
        .merge(api_routes(rate_limit, &cors_origins))
        .nest_service("/static", ServeDir::new("static"))
        .nest_service(
            "/app",
            ServeDir::new("spa/dist").fallback(ServeFile::new("spa/dist/index.html")),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
        .merge(ap_router)
}

async fn health_handler() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({ "status": "ok" }))
}

fn slow_down(reason: RejectionReason) -> axum::response::Response {
    match reason {
        RejectionReason::QuotaExceeded { .. } => refuse(
            StatusCode::TOO_MANY_REQUESTS,
            RATE_LIMITED,
            "too many requests: slow down and try again shortly",
        ),
        RejectionReason::KeyExtractionFailed(_) => refuse(
            StatusCode::INTERNAL_SERVER_ERROR,
            INTERNAL,
            "the caller could not be identified for rate limiting",
        ),
    }
}

fn api_governor(quota: Quota) -> axum_governor::GovernorConfig<std::net::IpAddr> {
    GovernorConfigBuilder::default()
        .with_extractor(PeerIp::default())
        .expect_connect_info()
        .quota_default(quota)
        .error_handler(slow_down)
        .finish()
        .unwrap()
}

fn per_minute(n: u64) -> Quota {
    let n = NonZeroU32::new(n.clamp(1, u32::MAX as u64) as u32).unwrap();
    Quota::requests_per_minute(n)
}

fn html_routes(rate_limit: u64, secure_cookies: bool) -> Router<AppState> {
    let auth = html::auth_router().layer({
        let cfg = GovernorConfigBuilder::default()
            .with_extractor(PeerIp::default())
            .expect_connect_info()
            .quota_default(per_minute(rate_limit))
            .finish()
            .unwrap();
        GovernorLayer::new(cfg)
    });

    html::router()
        .merge(federation::html_routes())
        .merge(auth)
        .layer(axum::middleware::from_fn_with_state(
            crate::html::csrf::SecureCookies(secure_cookies),
            crate::html::csrf::csrf_middleware,
        ))
}

fn cors_layer(origins: &str) -> CorsLayer {
    use axum::http::{HeaderName, Method};
    use tower_http::cors::AllowOrigin;

    let layer = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            HeaderName::from_static("content-type"),
            HeaderName::from_static("authorization"),
        ]);

    if origins.is_empty() || origins == "*" {
        layer.allow_origin(AllowOrigin::any())
    } else {
        let parsed: Vec<_> = origins
            .split(',')
            .filter_map(|s| {
                let trimmed = s.trim();
                match trimmed.parse() {
                    Ok(v) => Some(v),
                    Err(e) => {
                        tracing::warn!("ignoring invalid CORS origin {trimmed:?}: {e}");
                        None
                    }
                }
            })
            .collect();
        layer
            .allow_origin(AllowOrigin::list(parsed))
            .allow_credentials(true)
    }
}

fn api_routes(rate_limit: u64, cors_origins: &str) -> Router<AppState> {
    let cfg = api_governor(per_minute(rate_limit));

    let base = api::router().merge(federation::api_routes());

    let webhook_cfg = api_governor(per_minute(rate_limit / 4));
    let webhook_routes = api::webhook_router().layer(GovernorLayer::new(webhook_cfg));

    Router::new()
        .nest("/api/v1", base.layer(GovernorLayer::new(cfg)))
        .nest("/api/v1", webhook_routes)
        .layer(cors_layer(cors_origins))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    async fn body_json(resp: axum::response::Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn api_rate_limit_refusal_is_the_one_error_shape() {
        let app = Router::new()
            .route("/x", routing::get(|| async { "ok" }))
            .layer(GovernorLayer::new(api_governor(per_minute(1))));
        let addr: std::net::SocketAddr = "127.0.0.1:1".parse().unwrap();
        let mut last = None;
        for _ in 0..3 {
            let mut req = Request::builder().uri("/x").body(Body::empty()).unwrap();
            req.extensions_mut()
                .insert(axum::extract::ConnectInfo::<std::net::SocketAddr>(addr));
            last = Some(app.clone().oneshot(req).await.unwrap());
        }
        let resp = last.unwrap();
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        let body = body_json(resp).await;
        assert_eq!(body["code"], "RATE_LIMITED");
        assert!(body["error"].is_string());
    }

    #[tokio::test]
    async fn unidentifiable_caller_is_refused_in_the_one_error_shape() {
        let resp = slow_down(RejectionReason::KeyExtractionFailed(
            axum_governor::ExtractionError::MissingConnectInfo,
        ));
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body_json(resp).await["code"], "INTERNAL");
    }
}
