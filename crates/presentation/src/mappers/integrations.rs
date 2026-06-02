use domain::models::{WatchEvent, WebhookToken};
use template_askama::{WatchQueueDisplayEntry, WebhookTokenView};

pub fn webhook_token_view(t: &WebhookToken) -> WebhookTokenView {
    WebhookTokenView {
        id: t.id().value().to_string(),
        provider: t.provider().to_string(),
        label: t.label().map(String::from),
        created_at: t.created_at().format("%Y-%m-%d %H:%M").to_string(),
        last_used_at: t
            .last_used_at()
            .map(|d| d.format("%Y-%m-%d %H:%M").to_string()),
    }
}

pub fn watch_queue_entry(e: &WatchEvent) -> WatchQueueDisplayEntry {
    WatchQueueDisplayEntry {
        id: e.id().value().to_string(),
        title: e.title().to_string(),
        year: e.year(),
        source: e.source().to_string(),
        watched_at: e.watched_at().format("%Y-%m-%d %H:%M").to_string(),
        movie_url: e.movie_id().map(|m| format!("/movies/{}", m.value())),
    }
}
