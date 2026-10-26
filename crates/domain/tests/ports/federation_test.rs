use domain::ports::noop::{NoopApBackfill, NoopApDocument, NoopInstanceBlocklist};
use domain::ports::{ApBackfillPort, ApDocumentPort, InstanceBlocklistPort};

#[tokio::test]
async fn noop_document_returns_empty_strings() {
    let p = NoopApDocument;
    assert_eq!(p.actor_json("anything").await.unwrap(), "");
    assert_eq!(
        p.followers_collection_json(uuid::Uuid::nil(), None)
            .await
            .unwrap(),
        ""
    );
    assert_eq!(
        p.following_collection_json(uuid::Uuid::nil(), Some(2))
            .await
            .unwrap(),
        ""
    );
}

#[tokio::test]
async fn noop_blocklist_returns_empty_and_ok() {
    let p = NoopInstanceBlocklist;
    assert!(p.get_blocked_domains().await.unwrap().is_empty());
    p.add_blocked_domain("evil.example", Some("spam"))
        .await
        .unwrap();
    p.remove_blocked_domain("evil.example").await.unwrap();
}

#[tokio::test]
async fn noop_backfill_returns_empty_and_ok() {
    let p = NoopApBackfill;
    assert!(p.get_following(uuid::Uuid::nil()).await.unwrap().is_empty());
    p.import_remote_outbox("https://a/outbox", "https://a")
        .await
        .unwrap();
    p.run_backfill_for_follower(uuid::Uuid::nil(), "https://a/inbox".into())
        .await
        .unwrap();
}
