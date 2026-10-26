mod support;

use axum::{
    Router,
    body::Body,
    http::{Method, Request, header},
};
use support::{get, json_request, send};

const EMAIL: &str = "owner@example.com";
const USERNAME: &str = "owner";

const CLASSIC_VISITOR_PAGES: &[(&str, u16)] = &[
    ("/", 200),
    ("/users", 200),
    ("/u/{username}", 308),
    ("/users/{id}", 200),
    ("/users/{id}?view=ratings", 200),
    ("/users/{id}?view=history", 200),
    ("/movies/{movie}", 200),
    ("/login", 200),
    ("/register", 200),
    ("/logout", 303),
    ("/tags/drama", 307),
    ("/users/{id}/watchlist", 200),
    ("/wrapups/{id}/2025", 200),
    ("/wrapups/global/2025", 200),
    ("/users/{id}/following-list", 303),
    ("/users/{id}/followers-list", 303),
    ("/posters/abc.jpg", 308),
    ("/images/missing.jpg", 404),
    ("/health", 200),
];

const CLASSIC_OWNER_PAGES: &[(&str, u16)] = &[
    ("/reviews/new", 200),
    ("/diary/export", 200),
    ("/import", 200),
    ("/import/done", 200),
    ("/import/00000000-0000-0000-0000-000000000000/mapping", 303),
    ("/import/00000000-0000-0000-0000-000000000000/preview", 303),
    ("/settings/profile", 200),
    ("/settings/integrations", 200),
    ("/watch-queue", 200),
    ("/users/{id}/watchlist", 200),
    ("/users/{id}/following-list", 200),
    ("/users/{id}/followers-list", 200),
    ("/movies/{movie}", 200),
    ("/users/{id}?embed=true", 200),
];

#[cfg(feature = "federation")]
const FEDERATION_VISITOR_PAGES: &[(&str, u16)] = &[
    ("/users/{id}/followers", 303),
    ("/users/{id}/following", 303),
];

#[cfg(feature = "federation")]
const FEDERATION_OWNER_PAGES: &[(&str, u16)] = &[
    ("/users/{id}/followers", 303),
    ("/users/{id}/following", 303),
    ("/admin/blocked-domains", 403),
    ("/social/blocked", 200),
];

#[cfg(not(feature = "federation"))]
const FEDERATION_OFF_PAGES: &[(&str, u16)] = &[
    ("/users/{id}/followers", 308),
    ("/users/{id}/following", 308),
];

struct Seed {
    router: Router,
    user_id: String,
    movie_id: String,
    owner_cookie: String,
}

async fn seed() -> Seed {
    let (router, pool) = support::server().await;
    let session = support::register(&router, USERNAME, EMAIL).await;
    let (token, user_id, owner_cookie) = (session.token, session.user_id, session.cookie);

    let (status, _, body) = send(
        &router,
        json_request(
            Method::POST,
            "/api/v1/reviews",
            Some(&token),
            serde_json::json!({
                "manual_title": "Seeded Movie",
                "manual_release_year": 1999,
                "manual_director": "Some Director",
                "rating": 4,
                "comment": "the butler did it",
                "watched_at": "2025-03-01",
                "spoilers": true,
            }),
        ),
    )
    .await;
    assert_eq!(status, 201, "log review: {body}");

    let (status, _, body) = send(
        &router,
        json_request(
            Method::POST,
            "/api/v1/goals",
            Some(&token),
            serde_json::json!({ "year": 2025, "target_count": 12 }),
        ),
    )
    .await;
    assert_eq!(status, 200, "create goal: {body}");

    let (status, _, body) = send(
        &router,
        Request::builder()
            .uri("/api/v1/movies")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, 200, "list movies: {body}");
    let movies: serde_json::Value = serde_json::from_str(&body).unwrap();
    let movie_id = movies["items"][0]["id"]
        .as_str()
        .unwrap_or_else(|| panic!("no movie id in {body}"))
        .to_string();

    seed_ready_wrapup(&pool, Some(&user_id)).await;
    seed_ready_wrapup(&pool, None).await;

    Seed {
        router,
        user_id,
        movie_id,
        owner_cookie,
    }
}

async fn seed_ready_wrapup(pool: &sqlx::SqlitePool, user_id: Option<&str>) {
    use chrono::NaiveDate;
    use domain::models::wrapup::{DateRange, WrapUpScope};

    let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let end = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    let scope = match user_id {
        Some(id) => WrapUpScope::User(id.parse().unwrap()),
        None => WrapUpScope::Global,
    };
    let report = domain::services::wrapup_analyzer::build_report(
        scope,
        DateRange::new(start, end).unwrap(),
        &[],
    );
    sqlx::query(
        "INSERT INTO wrap_up_records (id, user_id, start_date, end_date, status, report_json) \
         VALUES (?, ?, '2025-01-01', '2026-01-01', 'ready', ?)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(user_id)
    .bind(serde_json::to_string(&report).unwrap())
    .execute(pool)
    .await
    .unwrap();
}

fn expand(seed: &Seed, template: &str) -> String {
    template
        .replace("{id}", &seed.user_id)
        .replace("{username}", USERNAME)
        .replace("{movie}", &seed.movie_id)
}

async fn check_table(seed: &Seed, table: &[(&str, u16)], cookie: Option<&str>) {
    let mut failures = Vec::new();
    for (template, expected) in table {
        let uri = expand(seed, template);
        let (status, _, _) = send(&seed.router, get(&uri, cookie)).await;
        if status != *expected {
            failures.push(format!("{uri}: expected {expected}, got {status}"));
        }
    }
    assert!(
        failures.is_empty(),
        "route status drift:\n{}",
        failures.join("\n")
    );
}

#[tokio::test]
async fn every_classic_page_renders_for_a_visitor() {
    let seed = seed().await;
    check_table(&seed, CLASSIC_VISITOR_PAGES, None).await;
    #[cfg(feature = "federation")]
    check_table(&seed, FEDERATION_VISITOR_PAGES, None).await;
    #[cfg(not(feature = "federation"))]
    check_table(&seed, FEDERATION_OFF_PAGES, None).await;

    for template in ["/wrapups/{id}/2025", "/wrapups/global/2025"] {
        let (status, _, body) = send(&seed.router, get(&expand(&seed, template), None)).await;
        assert_eq!(status, 200, "{template}");
        assert!(
            body.contains("class=\"wu-container\""),
            "wrapup template not rendered for {template}"
        );
    }

    let (status, _, body) = send(
        &seed.router,
        get(&expand(&seed, "/users/{id}?embed=true"), None),
    )
    .await;
    assert_eq!(status, 200);
    assert!(
        body.contains("class=\"embed-profile\""),
        "embed template not rendered: {body}"
    );
}

#[tokio::test]
async fn every_classic_page_renders_for_the_signed_in_owner() {
    let seed = seed().await;
    let cookie = seed.owner_cookie.clone();
    check_table(&seed, CLASSIC_OWNER_PAGES, Some(&cookie)).await;
    #[cfg(feature = "federation")]
    check_table(&seed, FEDERATION_OWNER_PAGES, Some(&cookie)).await;
}

#[tokio::test]
async fn feeds_and_openapi_are_served() {
    let seed = seed().await;
    for (template, expected) in [
        ("/feed.rss", 200),
        ("/users/{id}/feed.rss", 200),
        ("/openapi.json", 200),
        ("/docs", 303),
        ("/docs/", 200),
    ] {
        let uri = expand(&seed, template);
        let (status, _, _) = send(&seed.router, get(&uri, None)).await;
        assert_eq!(status, expected, "{uri}");
    }
}

#[tokio::test]
async fn openapi_paths_match_the_recorded_list() {
    let seed = seed().await;
    let (status, _, body) = send(&seed.router, get("/openapi.json", None)).await;
    assert_eq!(status, 200);
    let spec: serde_json::Value = serde_json::from_str(&body).unwrap();
    let mut lines = Vec::new();
    for (path, item) in spec["paths"].as_object().unwrap() {
        for method in item.as_object().unwrap().keys() {
            if ["get", "put", "post", "delete", "patch", "head", "options"]
                .contains(&method.as_str())
            {
                lines.push(format!("{} {path}", method.to_uppercase()));
            }
        }
    }
    lines.sort();
    let actual = lines.join("\n") + "\n";

    let fixture = if cfg!(feature = "federation") {
        "tests/fixtures/openapi_paths.txt"
    } else {
        "tests/fixtures/openapi_paths_federation_off.txt"
    };
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(fixture);
    if std::env::var("UPDATE_OPENAPI_FIXTURE").is_ok() {
        std::fs::write(&path, &actual).unwrap();
    }
    let expected = std::fs::read_to_string(&path).unwrap();
    assert_eq!(actual, expected, "OpenAPI paths drifted from {fixture}");
}

#[cfg(feature = "federation")]
#[tokio::test]
async fn profile_page_serves_the_actor_document_to_activitypub_clients() {
    let seed = seed().await;
    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/users/{}", seed.user_id))
        .header(header::ACCEPT, "application/activity+json")
        .body(Body::empty())
        .unwrap();
    let (status, headers, body) = send(&seed.router, req).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        headers.get(header::CONTENT_TYPE).unwrap(),
        "application/activity+json"
    );
    let actor: serde_json::Value = serde_json::from_str(&body).expect("actor JSON");
    assert_eq!(actor["type"], "Person");
}
