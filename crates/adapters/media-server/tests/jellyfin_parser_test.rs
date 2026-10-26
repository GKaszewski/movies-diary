use domain::ports::MediaServerParser;
use media_server::JellyfinParser;

#[test]
fn parses_valid_playback_stop() {
    let body = serde_json::json!({
        "NotificationType": "PlaybackStop",
        "ItemType": "Movie",
        "Name": "Blade Runner",
        "Year": 1982,
        "PlayedToCompletion": true,
        "Provider_tmdb": "78",
        "Provider_imdb": "tt0083658"
    });
    let parser = JellyfinParser;
    let result = parser
        .parse_playback_event(serde_json::to_vec(&body).unwrap().as_slice())
        .unwrap();
    let event = result.expect("should parse");
    assert_eq!(event.title, "Blade Runner");
    assert_eq!(event.year, Some(1982));
    assert_eq!(event.tmdb_id, Some("tmdb:78".into()));
    assert_eq!(event.imdb_id, Some("tt0083658".into()));
}

#[test]
fn ignores_non_movie() {
    let body = serde_json::json!({
        "NotificationType": "PlaybackStop",
        "ItemType": "Episode",
        "Name": "Some Episode",
        "PlayedToCompletion": true
    });
    let parser = JellyfinParser;
    let result = parser
        .parse_playback_event(serde_json::to_vec(&body).unwrap().as_slice())
        .unwrap();
    assert!(result.is_none());
}

#[test]
fn ignores_incomplete_playback() {
    let body = serde_json::json!({
        "NotificationType": "PlaybackStop",
        "ItemType": "Movie",
        "Name": "Blade Runner",
        "PlayedToCompletion": false
    });
    let parser = JellyfinParser;
    let result = parser
        .parse_playback_event(serde_json::to_vec(&body).unwrap().as_slice())
        .unwrap();
    assert!(result.is_none());
}

#[test]
fn ignores_playback_start() {
    let body = serde_json::json!({
        "NotificationType": "PlaybackStart",
        "ItemType": "Movie",
        "Name": "Blade Runner",
        "PlayedToCompletion": false
    });
    let parser = JellyfinParser;
    let result = parser
        .parse_playback_event(serde_json::to_vec(&body).unwrap().as_slice())
        .unwrap();
    assert!(result.is_none());
}
