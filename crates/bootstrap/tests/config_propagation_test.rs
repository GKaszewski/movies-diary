mod support;

use axum::{
    body::Body,
    http::{Method, Request, header},
};

async fn server_with(config: config::AppConfig) -> axum::Router {
    bootstrap::build_server_on(&config, support::memory_pool().await)
        .await
        .expect("server builds")
}

fn set_cookies(headers: &axum::http::HeaderMap) -> Vec<String> {
    headers
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|v| v.to_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn registration_is_refused_when_allow_registration_is_false() {
    let mut config = support::test_config();
    config.server.allow_registration = false;
    let router = server_with(config).await;

    let form = Request::builder()
        .method(Method::POST)
        .uri("/register")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(format!(
            "email=a%40example.com&username=alice&password={}&_csrf=x",
            support::PASSWORD
        )))
        .unwrap();
    let (status, headers, _) = support::send(&router, form).await;
    assert_eq!(status, 303, "classic form redirects away");
    assert_eq!(headers.get(header::LOCATION).unwrap(), "/");
    assert!(
        support::cookie_value(&headers, "token").is_none(),
        "no session issued"
    );

    let (status, _, _) = support::send(
        &router,
        support::json_request(
            Method::POST,
            "/api/v1/auth/register",
            None,
            serde_json::json!({
                "email": "a@example.com",
                "username": "alice",
                "password": support::PASSWORD,
            }),
        ),
    )
    .await;
    assert_eq!(status, 401, "JSON register is refused");
}

#[tokio::test]
async fn secure_cookies_setting_marks_cookies_secure() {
    for secure in [true, false] {
        let mut config = support::test_config();
        config.server.secure_cookies = secure;
        let router = server_with(config).await;

        let (status, headers, _) = support::send(&router, support::get("/register", None)).await;
        assert_eq!(status, 200);
        let cookies = set_cookies(&headers);
        assert!(!cookies.is_empty(), "csrf cookie is set");
        assert!(
            cookies.iter().all(|c| c.contains("Secure") == secure),
            "secure_cookies={secure}: {cookies:?}"
        );

        let csrf = support::cookie_value(&headers, "csrf").expect("csrf cookie");
        let form = Request::builder()
            .method(Method::POST)
            .uri("/register")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .header(header::COOKIE, format!("csrf={csrf}"))
            .body(Body::from(format!(
                "email=a%40example.com&username=alice&password={}&_csrf={csrf}",
                support::PASSWORD
            )))
            .unwrap();
        let (status, headers, _) = support::send(&router, form).await;
        assert_eq!(status, 303);
        let token_cookie = set_cookies(&headers)
            .into_iter()
            .find(|c| c.starts_with("token="))
            .expect("session cookie");
        assert_eq!(
            token_cookie.contains("Secure"),
            secure,
            "session cookie, secure_cookies={secure}: {token_cookie}"
        );
    }
}
