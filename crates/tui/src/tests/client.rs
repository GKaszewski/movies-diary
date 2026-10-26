use super::*;

#[test]
fn api_client_builds_versioned_urls() {
    let client = ApiClient::new("http://localhost:3000");
    assert_eq!(client.api("/diary"), "http://localhost:3000/api/v1/diary");
    assert_eq!(
        client.api("/auth/login"),
        "http://localhost:3000/api/v1/auth/login"
    );
    assert_eq!(
        client.api("/social/follow"),
        "http://localhost:3000/api/v1/social/follow"
    );
}

#[test]
fn api_client_update_url() {
    let client = ApiClient::new("http://localhost:3000");
    assert!(client.url().contains("3000"));
    client.update_url("http://localhost:8080");
    assert!(client.url().contains("8080"));
    assert_eq!(client.api("/diary"), "http://localhost:8080/api/v1/diary");
}
