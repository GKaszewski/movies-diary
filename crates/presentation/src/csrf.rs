use axum::{
    extract::Request,
    http::{HeaderValue, header},
    middleware::Next,
    response::Response,
};

#[derive(Clone)]
pub struct CsrfToken(pub String);

pub fn extract_from_cookie(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies
                .split(';')
                .find_map(|c| c.trim().strip_prefix("csrf=").map(str::to_string))
        })
}

fn secure_flag() -> &'static str {
    if std::env::var("SECURE_COOKIES").as_deref() == Ok("true") {
        "; Secure"
    } else {
        ""
    }
}

pub async fn csrf_middleware(mut req: Request, next: Next) -> Response {
    let existing = extract_from_cookie(req.headers());
    let (token, needs_set) = match existing {
        Some(t) => (t, false),
        None => (uuid::Uuid::new_v4().to_string(), true),
    };

    req.extensions_mut().insert(CsrfToken(token.clone()));

    let mut response = next.run(req).await;

    if needs_set {
        let cookie = format!(
            "csrf={}; HttpOnly; Path=/; SameSite=Strict{}",
            token,
            secure_flag()
        );
        if let Ok(val) = HeaderValue::from_str(&cookie) {
            response.headers_mut().append(header::SET_COOKIE, val);
        }
    }

    response
}

/// Returns true if the form token does not match the cookie token.
pub fn mismatch(token: &CsrfToken, form_value: &str) -> bool {
    token.0 != form_value || form_value.is_empty()
}
