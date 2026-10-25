use super::*;
use domain::models::{Movie, Review};
use domain::value_objects::{Comment, MovieTitle, Rating, ReleaseYear, UserId};

fn channel<'a>(title: &'a str) -> FeedChannel<'a> {
    FeedChannel {
        title,
        link_path: "/users/abc",
        self_path: "/users/abc/feed.rss",
    }
}

fn entry(spoilers: bool) -> DiaryEntry {
    let movie = Movie::new(
        None,
        MovieTitle::new("Blade Runner".into()).unwrap(),
        ReleaseYear::new(1982).unwrap(),
        None,
        None,
    );
    let review = Review::new(
        movie.id().clone(),
        UserId::generate(),
        Rating::new(4).unwrap(),
        Some(Comment::new("replicants".into()).unwrap()),
        chrono::Utc::now().naive_utc(),
        None,
        spoilers,
    )
    .unwrap();
    DiaryEntry::new(movie, review)
}

#[test]
fn render_feed_uses_provided_title() {
    let adapter = RssAdapter::new("http://example.com".into());
    let xml = adapter.render_feed(&[], &channel("Custom Title")).unwrap();
    assert!(xml.contains("<title>Custom Title</title>"));
}

#[test]
fn render_feed_empty_entries_produces_valid_xml() {
    let adapter = RssAdapter::new("http://example.com".into());
    let xml = adapter.render_feed(&[], &channel("My Feed")).unwrap();
    assert!(xml.starts_with("<?xml") || xml.starts_with("<rss"));
}

#[test]
fn channel_links_are_absolute_with_atom_self() {
    let adapter = RssAdapter::new("http://example.com/".into());
    let xml = adapter.render_feed(&[], &channel("Feed")).unwrap();
    assert!(xml.contains("<link>http://example.com/users/abc</link>"));
    assert!(xml.contains(r#"xmlns:atom="http://www.w3.org/2005/Atom""#));
    assert!(xml.contains(r#"href="http://example.com/users/abc/feed.rss""#));
    assert!(xml.contains(r#"rel="self""#));
    assert!(xml.contains("<language>en</language>"));
}

#[test]
fn item_links_to_absolute_movie_page() {
    let adapter = RssAdapter::new("http://example.com".into());
    let e = entry(false);
    let xml = adapter.render_feed(&[e.clone()], &channel("Feed")).unwrap();
    let expected = format!(
        "<link>http://example.com/movies/{}</link>",
        e.movie().id().value()
    );
    assert!(xml.contains(&expected), "{xml}");
}

#[test]
fn spoiler_comment_is_omitted() {
    let adapter = RssAdapter::new("http://example.com".into());
    let xml = adapter
        .render_feed(&[entry(true)], &channel("Feed"))
        .unwrap();
    assert!(xml.contains("contains spoilers"));
    assert!(!xml.contains("replicants"));
}
