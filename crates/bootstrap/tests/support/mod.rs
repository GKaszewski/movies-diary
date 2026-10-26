#![allow(dead_code)]

use axum::{
    Router,
    body::Body,
    http::{Method, Request, header},
};
use tower::ServiceExt;

pub const PASSWORD: &str = "correct-horse-battery";

pub fn test_config() -> config::AppConfig {
    config::AppConfig {
        server: config::ServerConfig {
            allow_registration: true,
            base_url: "http://localhost:3000".into(),
            ..Default::default()
        },
        database: config::DatabaseConfig {
            url: "sqlite::memory:".into(),
        },
        auth: config::AuthConfig {
            jwt_secret: Some("route-smoke-test-secret-0123456789abcdef".into()),
            ..Default::default()
        },
        events: Default::default(),
        nats: Default::default(),
        storage: Default::default(),
        image_conversion: Default::default(),
        metadata: config::MetadataConfig {
            tmdb_api_key: Some("test-key".into()),
            ..Default::default()
        },
        federation: Default::default(),
        wrapup: Default::default(),
    }
}

pub async fn send(router: &Router, mut req: Request<Body>) -> (u16, axum::http::HeaderMap, String) {
    let addr: std::net::SocketAddr = "127.0.0.1:12345".parse().unwrap();
    req.extensions_mut()
        .insert(axum::extract::ConnectInfo::<std::net::SocketAddr>(addr));
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status().as_u16();
    let headers = resp.headers().clone();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        headers,
        String::from_utf8_lossy(&bytes).into_owned(),
    )
}

pub fn get(uri: &str, cookie: Option<&str>) -> Request<Body> {
    let mut b = Request::builder().method(Method::GET).uri(uri);
    if let Some(c) = cookie {
        b = b.header(header::COOKIE, c);
    }
    b.body(Body::empty()).unwrap()
}

pub fn cookie_value(headers: &axum::http::HeaderMap, name: &str) -> Option<String> {
    headers
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find_map(|c| {
            c.split(';')
                .next()?
                .trim()
                .strip_prefix(&format!("{name}="))
                .map(str::to_string)
        })
}

pub fn json_request(
    method: Method,
    uri: &str,
    token: Option<&str>,
    body: serde_json::Value,
) -> Request<Body> {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(t) = token {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    b.body(Body::from(body.to_string())).unwrap()
}

pub async fn memory_pool() -> sqlx::SqlitePool {
    sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap()
}

pub async fn server() -> (Router, sqlx::SqlitePool) {
    let pool = memory_pool().await;
    let router = bootstrap::build_server_on(&test_config(), pool.clone())
        .await
        .unwrap();
    (router, pool)
}

pub struct Session {
    pub user_id: String,
    pub token: String,
    pub cookie: String,
}

pub async fn register(router: &Router, username: &str, email: &str) -> Session {
    let (status, headers, _) = send(router, get("/register", None)).await;
    assert_eq!(status, 200, "GET /register");
    let csrf = cookie_value(&headers, "csrf").expect("csrf cookie");
    let form = format!(
        "email={}&username={username}&password={PASSWORD}&_csrf={csrf}",
        email.replace('@', "%40")
    );
    let req = Request::builder()
        .method(Method::POST)
        .uri("/register")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(header::COOKIE, format!("csrf={csrf}"))
        .body(Body::from(form))
        .unwrap();
    let (status, headers, _) = send(router, req).await;
    assert_eq!(status, 303, "POST /register");
    let session = cookie_value(&headers, "token").expect("session cookie after register");
    let cookie = format!("csrf={csrf}; token={session}");

    let (status, _, body) = send(
        router,
        json_request(
            Method::POST,
            "/api/v1/auth/login",
            None,
            serde_json::json!({ "email": email, "password": PASSWORD }),
        ),
    )
    .await;
    assert_eq!(status, 200, "login: {body}");
    let login: serde_json::Value = serde_json::from_str(&body).unwrap();
    Session {
        user_id: login["user_id"].as_str().unwrap().to_string(),
        token: login["token"].as_str().unwrap().to_string(),
        cookie,
    }
}
