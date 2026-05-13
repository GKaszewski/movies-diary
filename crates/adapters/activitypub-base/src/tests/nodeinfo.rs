use super::*;

#[test]
fn nodeinfo_well_known_serializes_correctly() {
    let doc = NodeInfoWellKnown {
        links: vec![NodeInfoLink {
            rel: "http://nodeinfo.diaspora.software/ns/schema/2.0".to_string(),
            href: "https://example.com/nodeinfo/2.0".to_string(),
        }],
    };
    let json = serde_json::to_value(&doc).unwrap();
    assert_eq!(
        json["links"][0]["rel"],
        "http://nodeinfo.diaspora.software/ns/schema/2.0"
    );
    assert_eq!(json["links"][0]["href"], "https://example.com/nodeinfo/2.0");
}

#[test]
fn nodeinfo_serializes_camel_case() {
    let doc = NodeInfo {
        version: "2.0".to_string(),
        software: NodeInfoSoftware {
            name: "my-app".to_string(),
            version: "0.1.0".to_string(),
        },
        protocols: vec!["activitypub".to_string()],
        usage: NodeInfoUsage {
            users: NodeInfoUsers { total: 3 },
            local_posts: 42,
        },
        open_registrations: false,
    };
    let json = serde_json::to_value(&doc).unwrap();
    assert_eq!(json["version"], "2.0");
    assert_eq!(json["software"]["name"], "my-app");
    assert_eq!(json["usage"]["users"]["total"], 3);
    assert_eq!(json["usage"]["localPosts"], 42);
    assert_eq!(json["openRegistrations"], false);
}
