use super::*;

#[test]
fn render_feed_uses_provided_title() {
    let adapter = RssAdapter::new("http://example.com".into());
    let xml = adapter.render_feed(&[], "Custom Title").unwrap();
    assert!(xml.contains("<title>Custom Title</title>"));
}

#[test]
fn render_feed_empty_entries_produces_valid_xml() {
    let adapter = RssAdapter::new("http://example.com".into());
    let xml = adapter.render_feed(&[], "My Feed").unwrap();
    assert!(xml.starts_with("<?xml") || xml.starts_with("<rss"));
}
