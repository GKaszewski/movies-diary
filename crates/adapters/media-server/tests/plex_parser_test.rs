use domain::ports::MediaServerParser;
use media_server::PlexParser;

#[test]
fn parses_movie_scrobble() {
    let body = serde_json::json!({
        "event": "media.scrobble",
        "Metadata": {
            "type": "movie",
            "title": "Blade Runner",
            "year": 1982,
            "Guid": [
                {"id": "tmdb://78"},
                {"id": "imdb://tt0083658"}
            ]
        }
    });
    let parser = PlexParser;
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
fn ignores_tv_episode() {
    let body = serde_json::json!({
        "event": "media.scrobble",
        "Metadata": {
            "type": "episode",
            "title": "Pilot",
            "grandparentTitle": "Breaking Bad",
            "year": 2008,
            "Guid": []
        }
    });
    let parser = PlexParser;
    let result = parser
        .parse_playback_event(serde_json::to_vec(&body).unwrap().as_slice())
        .unwrap();
    assert!(result.is_none());
}

#[test]
fn ignores_play_event() {
    let body = serde_json::json!({
        "event": "media.play",
        "Metadata": {
            "type": "movie",
            "title": "Blade Runner",
            "year": 1982,
            "Guid": []
        }
    });
    let parser = PlexParser;
    let result = parser
        .parse_playback_event(serde_json::to_vec(&body).unwrap().as_slice())
        .unwrap();
    assert!(result.is_none());
}

#[test]
fn handles_no_guids() {
    let body = serde_json::json!({
        "event": "media.scrobble",
        "Metadata": {
            "type": "movie",
            "title": "Some Indie Film",
            "year": 2023
        }
    });
    let parser = PlexParser;
    let result = parser
        .parse_playback_event(serde_json::to_vec(&body).unwrap().as_slice())
        .unwrap();
    let event = result.expect("should parse");
    assert_eq!(event.title, "Some Indie Film");
    assert!(event.tmdb_id.is_none());
    assert!(event.imdb_id.is_none());
}

#[test]
fn handles_missing_metadata() {
    let body = serde_json::json!({
        "event": "media.scrobble"
    });
    let parser = PlexParser;
    let result = parser
        .parse_playback_event(serde_json::to_vec(&body).unwrap().as_slice())
        .unwrap();
    assert!(result.is_none());
}
