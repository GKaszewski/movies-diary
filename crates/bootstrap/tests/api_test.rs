mod support;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use sqlx::SqlitePool;
use tower::ServiceExt;
use uuid::Uuid;

async fn test_app() -> Router {
    support::server().await.0
}

async fn test_app_with_user() -> (Router, SqlitePool, Uuid, String) {
    let (app, pool) = support::server().await;
    let session = support::register(&app, "insights", "insights@example.com").await;
    let user_id = session.user_id.parse().unwrap();
    (app, pool, user_id, session.token)
}

/// Inject a fake peer IP so the GovernorLayer can extract ConnectInfo.
fn with_ip(req: Request<Body>) -> Request<Body> {
    let addr: std::net::SocketAddr = "127.0.0.1:12345".parse().unwrap();
    let mut req = req;
    req.extensions_mut()
        .insert(axum::extract::ConnectInfo::<std::net::SocketAddr>(addr));
    req
}

#[tokio::test]
async fn get_api_diary_returns_empty_list() {
    let app = test_app().await;
    let response = app
        .oneshot(with_ip(
            Request::builder()
                .uri("/api/v1/diary")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(json["total_count"], 0);
    assert_eq!(json["items"], serde_json::json!([]));
    assert_eq!(json["limit"], 5);
    assert_eq!(json["offset"], 0);
}

#[tokio::test]
async fn post_api_reviews_without_auth_returns_401() {
    let app = test_app().await;
    let response = app
        .oneshot(with_ip(
            Request::builder()
                .method("POST")
                .uri("/api/v1/reviews")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"rating":4,"watched_at":"2026-01-01T20:00:00","manual_title":"Dune","manual_release_year":2021}"#,
                ))
                .unwrap(),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn post_api_auth_login_unknown_user_returns_401() {
    let app = test_app().await;
    let response = app
        .oneshot(with_ip(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"email":"a@b.com","password":"x"}"#))
                .unwrap(),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_api_movie_detail_returns_404_for_unknown_id() {
    let app = test_app().await;
    let response = app
        .oneshot(with_ip(
            Request::builder()
                .uri("/api/v1/movies/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn tags_moviesdiary_redirects_to_home() {
    let app = test_app().await;
    let response = app
        .oneshot(with_ip(
            Request::builder()
                .uri("/tags/moviesdiary")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(response.headers().get("location").unwrap(), "/");
}

#[tokio::test]
async fn tags_other_redirects_to_search() {
    let app = test_app().await;
    let response = app
        .oneshot(with_ip(
            Request::builder()
                .uri("/tags/batman")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(
        response.headers().get("location").unwrap(),
        "/?search=batman"
    );
}

#[tokio::test]
async fn get_movie_detail_html_returns_404_for_unknown_id() {
    let app = test_app().await;
    let response = app
        .oneshot(with_ip(
            Request::builder()
                .uri("/movies/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ── Insights ─────────────────────────────────────────────────────────────────

async fn seed_insights_diary(pool: &SqlitePool, user_id: Uuid) {
    // 12 thrillers rated 5 late at night, 12 comedies rated 2 — enough for a
    // rating lift once genres come from movie_genres.
    for i in 0..12u32 {
        let movie_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO movies (id, title, release_year) VALUES (?, ?, ?)")
            .bind(&movie_id)
            .bind(format!("Thriller {i}"))
            .bind(2020)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO movie_genres (movie_id, tmdb_id, name) VALUES (?, ?, ?)")
            .bind(&movie_id)
            .bind(44400 + i as i64)
            .bind("Thriller")
            .execute(pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO reviews (id, movie_id, user_id, rating, watched_at, created_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&movie_id)
        .bind(user_id.to_string())
        .bind(5)
        .bind("2024-01-05 22:30:00")
        .bind("2024-01-05 22:30:00")
        .execute(pool)
        .await
        .unwrap();

        let movie_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO movies (id, title, release_year) VALUES (?, ?, ?)")
            .bind(&movie_id)
            .bind(format!("Comedy {i}"))
            .bind(2015)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO movie_genres (movie_id, tmdb_id, name) VALUES (?, ?, ?)")
            .bind(&movie_id)
            .bind(55300 + i as i64)
            .bind("Comedy")
            .execute(pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO reviews (id, movie_id, user_id, rating, watched_at, created_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&movie_id)
        .bind(user_id.to_string())
        .bind(2)
        .bind("2024-03-10 13:00:00")
        .bind("2024-03-10 13:00:00")
        .execute(pool)
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn get_api_insights_returns_rating_lift_for_own_user() {
    let (app, pool, user_id, token) = test_app_with_user().await;
    seed_insights_diary(&pool, user_id).await;

    let response = app
        .oneshot(with_ip(
            Request::builder()
                .uri("/api/v1/insights")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();

    let (status, bytes) = (
        response.status(),
        axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    );
    assert_eq!(
        status,
        StatusCode::OK,
        "body: {}",
        String::from_utf8(bytes.to_vec()).unwrap()
    );
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    let rating_lifts: Vec<&serde_json::Value> = json["insights"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["kind"] == "rating_lift")
        .collect();
    assert_eq!(rating_lifts.len(), 2, "thriller up, comedy down: {json}");

    let thriller = rating_lifts
        .iter()
        .find(|i| i["genre"] == "Thriller")
        .unwrap();
    assert_eq!(thriller["delta"], 1.5);
    assert_eq!(thriller["sample_count"], 12);
    // Timestamps are real → rhythm fires. Hours split evenly between 22
    // (thrillers) and 13 (comedies); every window covering either bucket ties,
    // so the earliest one, 11:00–14:00, wins. Days tie too (Fri/Sun), sorted
    // ascending.
    let rhythm = json["insights"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["kind"] == "rhythm")
        .expect("rhythm expected for real timestamps");
    assert_eq!(rhythm["peak_hour_start"], 11);
    assert_eq!(rhythm["peak_hour_end"], 14);
    assert_eq!(rhythm["top_days"], serde_json::json!([5, 7]));
}

async fn get_insights_json(
    app: Router,
    token: &str,
    query: &str,
) -> (StatusCode, serde_json::Value) {
    let response = app
        .oneshot(with_ip(
            Request::builder()
                .uri(format!("/api/v1/insights{query}"))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, json)
}

#[tokio::test]
async fn get_api_insights_ignores_remote_reviews() {
    let (app, pool, user_id, token) = test_app_with_user().await;
    seed_insights_diary(&pool, user_id).await;
    // Federated rows stored under the same user_id are someone else's watches.
    for i in 0..12u32 {
        let movie_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO movies (id, title, release_year) VALUES (?, ?, ?)")
            .bind(&movie_id)
            .bind(format!("Horror {i}"))
            .bind(2019)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO movie_genres (movie_id, tmdb_id, name) VALUES (?, ?, ?)")
            .bind(&movie_id)
            .bind(66600 + i as i64)
            .bind("Horror")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO reviews (id, movie_id, user_id, rating, watched_at, created_at, remote_actor_url) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&movie_id)
        .bind(user_id.to_string())
        .bind(5)
        .bind("2024-05-05 09:00:00")
        .bind("2024-05-05 09:00:00")
        .bind("https://remote.example/users/someone")
        .execute(&pool)
        .await
        .unwrap();
    }

    let (status, json) = get_insights_json(app, &token, "").await;
    assert_eq!(status, StatusCode::OK);
    let insights = json["insights"].as_array().unwrap();
    assert!(
        !insights.iter().any(|i| i["genre"] == "Horror"),
        "remote reviews must not feed insights: {json}"
    );
    let thriller = insights
        .iter()
        .find(|i| i["kind"] == "rating_lift" && i["genre"] == "Thriller")
        .unwrap();
    assert_eq!(thriller["delta"], 1.5);
}

#[tokio::test]
async fn get_api_insights_buckets_rhythm_in_requested_time_zone() {
    let (app, pool, user_id, token) = test_app_with_user().await;
    seed_insights_diary(&pool, user_id).await;

    // January in Warsaw is UTC+1: 22:30 → 23:30 and 13:00 → 14:00, so the
    // earliest tied window moves from 11–14 to 12–15.
    let (status, json) = get_insights_json(app, &token, "?tz=Europe/Warsaw").await;
    assert_eq!(status, StatusCode::OK);
    let rhythm = json["insights"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["kind"] == "rhythm")
        .expect("rhythm expected");
    assert_eq!(rhythm["peak_hour_start"], 12);
    assert_eq!(rhythm["peak_hour_end"], 15);
}

#[tokio::test]
async fn get_api_insights_rejects_unknown_time_zone() {
    let (app, _pool, _user_id, token) = test_app_with_user().await;
    let (status, _) = get_insights_json(app, &token, "?tz=Mars/Olympus").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
