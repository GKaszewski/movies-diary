use super::*;

#[test]
fn person_serializes_with_enriched_fields() {
    let person = Person {
        kind: Default::default(),
        id: "https://example.com/users/1"
            .parse::<url::Url>()
            .unwrap()
            .into(),
        preferred_username: "alice".to_string(),
        inbox: "https://example.com/users/1/inbox".parse().unwrap(),
        outbox: "https://example.com/users/1/outbox".parse().unwrap(),
        followers: "https://example.com/users/1/followers".parse().unwrap(),
        following: "https://example.com/users/1/following".parse().unwrap(),
        public_key: PublicKey {
            id: "https://example.com/users/1#main-key".to_string(),
            owner: "https://example.com/users/1".parse().unwrap(),
            public_key_pem: "pem".to_string(),
        },
        name: Some("Alice".to_string()),
        summary: Some("Bio text".to_string()),
        icon: Some(ApImageObject {
            kind: "Image".to_string(),
            url: "https://example.com/images/avatars/1".parse().unwrap(),
        }),
        url: Some("https://example.com/u/alice".parse().unwrap()),
        discoverable: Some(true),
        manually_approves_followers: true,
        updated: Some(Utc::now()),
        endpoints: Some(Endpoints {
            shared_inbox: "https://example.com/inbox".parse().unwrap(),
        }),
        image: None,
        also_known_as: vec![],
        attachment: vec![],
    };
    let json = serde_json::to_value(&person).unwrap();
    assert_eq!(json["discoverable"], true);
    assert_eq!(json["summary"], "Bio text");
    assert_eq!(json["icon"]["type"], "Image");
    assert_eq!(json["manuallyApprovesFollowers"], true);
    assert!(json.get("updated").is_some());
    assert!(json.get("endpoints").is_some());
    assert_eq!(
        json["endpoints"]["sharedInbox"],
        "https://example.com/inbox"
    );
}
