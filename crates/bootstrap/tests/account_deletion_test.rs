mod support;

use axum::{
    body::Body,
    http::{Method, Request, header},
};
use support::{PASSWORD, Session, get, json_request, send};

async fn promote(pool: &sqlx::SqlitePool, user_id: &str) {
    sqlx::query("UPDATE users SET role = 'admin' WHERE id = ?")
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();
}

fn delete_json(uri: &str, token: &str, password: &str) -> Request<Body> {
    json_request(
        Method::DELETE,
        uri,
        Some(token),
        serde_json::json!({ "password": password }),
    )
}

fn classic_post(uri: &str, cookie: &str, password: &str) -> Request<Body> {
    let csrf = cookie
        .split(';')
        .find_map(|c| c.trim().strip_prefix("csrf="))
        .unwrap();
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(header::COOKIE, cookie)
        .body(Body::from(format!("password={password}&_csrf={csrf}")))
        .unwrap()
}

fn location(headers: &axum::http::HeaderMap) -> String {
    headers
        .get(header::LOCATION)
        .map(|v| v.to_str().unwrap().to_string())
        .unwrap_or_default()
}

async fn two_users() -> (axum::Router, sqlx::SqlitePool, Session, Session) {
    let (router, pool) = support::server().await;
    let alice = support::register(&router, "alice", "alice@example.com").await;
    let bob = support::register(&router, "bob", "bob@example.com").await;
    (router, pool, alice, bob)
}

#[tokio::test]
async fn self_delete_with_a_wrong_password_is_refused_and_keeps_the_account() {
    let (router, _pool, alice, _bob) = two_users().await;
    let (status, _, body) = send(
        &router,
        delete_json("/api/v1/users/me", &alice.token, "nope"),
    )
    .await;
    assert_eq!(status, 401, "{body}");
    assert!(body.contains("\"INVALID_CREDENTIALS\""), "{body}");
    let (status, _, _) = send(&router, get(&format!("/users/{}", alice.user_id), None)).await;
    assert_eq!(status, 200);
}

#[tokio::test]
async fn self_delete_removes_the_account_and_revokes_the_token() {
    let (router, _pool, alice, _bob) = two_users().await;
    let (status, _, body) = send(
        &router,
        delete_json("/api/v1/users/me", &alice.token, PASSWORD),
    )
    .await;
    assert_eq!(status, 204, "{body}");

    let req = Request::builder()
        .uri("/api/v1/profile")
        .header(header::AUTHORIZATION, format!("Bearer {}", alice.token))
        .body(Body::empty())
        .unwrap();
    let (status, _, _) = send(&router, req).await;
    assert_eq!(status, 401, "token of a deleted account");

    for uri in [
        format!("/users/{}", alice.user_id),
        format!("/users/{}?embed=true", alice.user_id),
    ] {
        let (status, _, _) = send(&router, get(&uri, None)).await;
        assert_eq!(status, 404, "{uri}");
    }
}

#[tokio::test]
async fn a_standard_user_cannot_delete_someone_else() {
    let (router, _pool, alice, bob) = two_users().await;
    let uri = format!("/api/v1/users/{}", bob.user_id);
    let (status, _, body) = send(&router, delete_json(&uri, &alice.token, PASSWORD)).await;
    assert_eq!(status, 403, "{body}");
    let (status, _, _) = send(&router, get(&format!("/users/{}", bob.user_id), None)).await;
    assert_eq!(status, 200);
}

#[tokio::test]
async fn an_admin_deletes_another_account() {
    let (router, pool, alice, bob) = two_users().await;
    promote(&pool, &alice.user_id).await;
    let uri = format!("/api/v1/users/{}", bob.user_id);

    let (status, _, body) = send(&router, delete_json(&uri, &alice.token, "nope")).await;
    assert_eq!(status, 401, "wrong admin password");
    assert!(body.contains("\"INVALID_CREDENTIALS\""), "{body}");

    let (status, _, body) = send(&router, delete_json(&uri, &alice.token, PASSWORD)).await;
    assert_eq!(status, 204, "{body}");

    for uri in [
        format!("/users/{}", bob.user_id),
        format!("/users/{}?embed=true", bob.user_id),
    ] {
        let (status, _, _) = send(&router, get(&uri, None)).await;
        assert_eq!(status, 404, "{uri}");
    }
    let (status, _, _) = send(&router, delete_json(&uri, &alice.token, PASSWORD)).await;
    assert_eq!(status, 404, "already deleted");

    #[cfg(feature = "federation")]
    {
        let req = Request::builder()
            .uri(format!("/users/{}", bob.user_id))
            .header(header::ACCEPT, "application/activity+json")
            .body(Body::empty())
            .unwrap();
        let (status, _, _) = send(&router, req).await;
        assert_eq!(status, 410, "actor endpoint of a deleted account");
    }
}

#[tokio::test]
async fn the_last_admin_cannot_be_deleted() {
    let (router, pool, alice, _bob) = two_users().await;
    promote(&pool, &alice.user_id).await;
    let (status, _, body) = send(
        &router,
        delete_json("/api/v1/users/me", &alice.token, PASSWORD),
    )
    .await;
    assert_eq!(status, 409, "{body}");
    let (status, _, _) = send(&router, get(&format!("/users/{}", alice.user_id), None)).await;
    assert_eq!(status, 200);
}

#[tokio::test]
async fn classic_self_delete_logs_out_and_redirects_home() {
    let (router, _pool, alice, _bob) = two_users().await;

    let (status, headers, _) = send(
        &router,
        classic_post("/settings/account/delete", &alice.cookie, "nope"),
    )
    .await;
    assert_eq!(status, 303);
    assert!(
        location(&headers).starts_with("/settings/profile?error="),
        "{headers:?}"
    );
    assert!(
        support::cookie_value(&headers, "token").is_none(),
        "a wrong password must not log out"
    );

    let (status, headers, _) = send(
        &router,
        classic_post("/settings/account/delete", &alice.cookie, PASSWORD),
    )
    .await;
    assert_eq!(status, 303);
    assert_eq!(location(&headers), "/");
    assert_eq!(
        support::cookie_value(&headers, "token").as_deref(),
        Some("")
    );

    let (status, _, _) = send(&router, get(&format!("/users/{}", alice.user_id), None)).await;
    assert_eq!(status, 404);
}

#[tokio::test]
async fn classic_self_delete_requires_the_csrf_token() {
    let (router, _pool, alice, _bob) = two_users().await;
    let req = Request::builder()
        .method(Method::POST)
        .uri("/settings/account/delete")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(header::COOKIE, &alice.cookie)
        .body(Body::from(format!("password={PASSWORD}&_csrf=wrong")))
        .unwrap();
    let (status, _, _) = send(&router, req).await;
    assert_eq!(status, 403);
}

#[tokio::test]
async fn classic_admin_delete_redirects_to_the_member_list() {
    let (router, pool, alice, bob) = two_users().await;
    let uri = format!("/users/{}/delete", bob.user_id);

    let (status, headers, _) = send(&router, classic_post(&uri, &alice.cookie, PASSWORD)).await;
    assert_eq!(status, 403, "non-admin: {headers:?}");

    promote(&pool, &alice.user_id).await;
    let (status, headers, _) = send(&router, classic_post(&uri, &alice.cookie, PASSWORD)).await;
    assert_eq!(status, 303);
    assert_eq!(location(&headers), "/users");
    assert!(support::cookie_value(&headers, "token").is_none());

    let (status, _, _) = send(&router, get(&format!("/users/{}", bob.user_id), None)).await;
    assert_eq!(status, 404);
}

#[tokio::test]
async fn classic_admin_delete_requires_the_csrf_token() {
    let (router, pool, alice, bob) = two_users().await;
    promote(&pool, &alice.user_id).await;
    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("/users/{}/delete", bob.user_id))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(header::COOKIE, &alice.cookie)
        .body(Body::from(format!("password={PASSWORD}&_csrf=wrong")))
        .unwrap();
    let (status, _, _) = send(&router, req).await;
    assert_eq!(status, 403);
    let (status, _, _) = send(&router, get(&format!("/users/{}", bob.user_id), None)).await;
    assert_eq!(status, 200, "target must survive a CSRF rejection");
}

#[tokio::test]
async fn a_non_admin_classic_delete_of_someone_else_is_forbidden() {
    let (router, _pool, alice, bob) = two_users().await;
    let uri = format!("/users/{}/delete", bob.user_id);
    let (status, _, _) = send(&router, classic_post(&uri, &alice.cookie, PASSWORD)).await;
    assert_eq!(status, 403);
    let (status, _, _) = send(&router, get(&format!("/users/{}", bob.user_id), None)).await;
    assert_eq!(status, 200);
}

#[tokio::test]
async fn classic_admin_self_delete_logs_out_unless_last_admin() {
    let (router, pool, alice, bob) = two_users().await;
    promote(&pool, &alice.user_id).await;
    let uri = format!("/users/{}/delete", alice.user_id);

    let (status, headers, _) = send(&router, classic_post(&uri, &alice.cookie, PASSWORD)).await;
    assert_eq!(status, 303);
    assert!(location(&headers).contains("error="), "{headers:?}");
    assert!(support::cookie_value(&headers, "token").is_none());
    let (status, _, _) = send(&router, get(&format!("/users/{}", alice.user_id), None)).await;
    assert_eq!(status, 200);

    promote(&pool, &bob.user_id).await;
    let (status, headers, _) = send(&router, classic_post(&uri, &alice.cookie, PASSWORD)).await;
    assert_eq!(status, 303);
    assert_eq!(location(&headers), "/");
    assert_eq!(
        support::cookie_value(&headers, "token").as_deref(),
        Some("")
    );
    let (status, _, _) = send(&router, get(&format!("/users/{}", alice.user_id), None)).await;
    assert_eq!(status, 404);
}

#[tokio::test]
async fn delete_forms_render_only_where_they_apply() {
    let (router, pool, alice, bob) = two_users().await;
    let own_delete = "action=\"/settings/account/delete\"";
    let user_delete = |id: &str| format!("action=\"/users/{id}/delete\"");

    let (_, _, body) = send(&router, get("/settings/profile", Some(&alice.cookie))).await;
    assert!(body.contains(own_delete), "owner settings page");

    let bob_page = format!("/users/{}", bob.user_id);
    let (_, _, body) = send(&router, get(&bob_page, None)).await;
    assert!(!body.contains(&user_delete(&bob.user_id)), "anonymous");
    let (_, _, body) = send(&router, get(&bob_page, Some(&alice.cookie))).await;
    assert!(!body.contains(&user_delete(&bob.user_id)), "non-admin");

    promote(&pool, &alice.user_id).await;
    let (_, _, body) = send(&router, get(&bob_page, Some(&alice.cookie))).await;
    assert!(body.contains(&user_delete(&bob.user_id)), "admin on other");

    let (_, _, body) = send(
        &router,
        get(&format!("/users/{}", alice.user_id), Some(&alice.cookie)),
    )
    .await;
    assert!(
        !body.contains(&user_delete(&alice.user_id)),
        "admin on self"
    );

    let (_, _, body) = send(
        &router,
        get(&format!("{bob_page}?embed=true"), Some(&alice.cookie)),
    )
    .await;
    assert!(!body.contains("/delete"), "embed page");
}

const SERVICE_ACTOR: &str = "00000000-0000-4000-8000-000000000000";

#[tokio::test]
async fn the_service_actor_is_not_deletable_and_offers_no_delete_ui() {
    let (router, pool, alice, bob) = two_users().await;
    promote(&pool, &alice.user_id).await;
    promote(&pool, &bob.user_id).await;

    let page = format!("/users/{SERVICE_ACTOR}");
    let (status, _, body) = send(&router, get(&page, Some(&alice.cookie))).await;
    assert_eq!(status, 200, "{body}");
    assert!(
        !body.contains(&format!("action=\"/users/{SERVICE_ACTOR}/delete\"")),
        "admin sees a delete form for the service actor"
    );

    let uri = format!("/api/v1/users/{SERVICE_ACTOR}");
    let (status, _, body) = send(&router, delete_json(&uri, &alice.token, PASSWORD)).await;
    assert_eq!(status, 403, "{body}");

    let req = Request::builder()
        .uri(&uri)
        .header(header::AUTHORIZATION, format!("Bearer {}", alice.token))
        .body(Body::empty())
        .unwrap();
    let (status, _, body) = send(&router, req).await;
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("\"is_service_account\":true"), "{body}");

    let req = Request::builder()
        .uri(format!("/api/v1/users/{}", bob.user_id))
        .header(header::AUTHORIZATION, format!("Bearer {}", alice.token))
        .body(Body::empty())
        .unwrap();
    let (_, _, body) = send(&router, req).await;
    assert!(!body.contains("is_service_account"), "{body}");
}

#[tokio::test]
async fn a_deleted_users_webhook_token_stops_ingesting_at_once() {
    let (router, pool, alice, _bob) = two_users().await;
    let (status, _, body) = send(
        &router,
        json_request(
            Method::POST,
            "/api/v1/settings/webhook-tokens",
            Some(&alice.token),
            serde_json::json!({ "provider": "jellyfin", "label": null }),
        ),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let token = serde_json::from_str::<serde_json::Value>(&body).unwrap()["token"]
        .as_str()
        .unwrap()
        .to_string();

    let hook = |token: &str| {
        Request::builder()
            .method(Method::POST)
            .uri("/api/v1/webhooks/jellyfin")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                serde_json::json!({
                    "NotificationType": "PlaybackStop", "ItemType": "Movie",
                    "Name": "Heat", "Year": 1995, "PlayedToCompletion": true,
                    "Provider_tmdb": "949"
                })
                .to_string(),
            ))
            .unwrap()
    };
    let events = || async {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM watch_events")
            .fetch_one(&pool)
            .await
            .unwrap()
    };

    let (status, _, body) = send(&router, hook(&token)).await;
    assert_eq!(status, 200, "live token: {body}");
    assert_eq!(events().await, 1);

    let (status, _, _) = send(
        &router,
        delete_json("/api/v1/users/me", &alice.token, PASSWORD),
    )
    .await;
    assert_eq!(status, 204);

    let (status, _, body) = send(&router, hook(&token)).await;
    assert_eq!(status, 401, "{body}");
    assert_eq!(events().await, 1, "no event ingested after deletion");
}
