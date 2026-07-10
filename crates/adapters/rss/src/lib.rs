use domain::models::DiaryEntry;
use domain::ports::RssFeedRenderer;
use rss_feed::{ChannelBuilder, GuidBuilder, ItemBuilder};

pub struct RssAdapter {
    feed_link: String,
}

impl RssAdapter {
    pub fn new(feed_link: String) -> Self {
        Self { feed_link }
    }
}

impl RssFeedRenderer for RssAdapter {
    fn render_feed(&self, entries: &[DiaryEntry], title: &str) -> Result<String, String> {
        let items = entries
            .iter()
            .map(|e| {
                let item_title = format!(
                    "{} ({})",
                    e.movie().title().value(),
                    e.movie().release_year().value()
                );
                let description = match e.review().comment() {
                    Some(c) => format!("{}/5 — {}", e.review().rating().value(), c.value()),
                    None => format!("{}/5", e.review().rating().value()),
                };
                let pub_date = e
                    .review()
                    .watched_at()
                    .and_utc()
                    .format("%a, %d %b %Y %H:%M:%S +0000")
                    .to_string();
                let guid = GuidBuilder::default()
                    .value(e.review().id().value().to_string())
                    .permalink(false)
                    .build();
                ItemBuilder::default()
                    .title(Some(item_title))
                    .description(Some(description))
                    .pub_date(Some(pub_date))
                    .guid(Some(guid))
                    .build()
            })
            .collect::<Vec<_>>();

        let channel = ChannelBuilder::default()
            .title(title.to_string())
            .link(self.feed_link.clone())
            .description(title.to_string())
            .items(items)
            .build();

        Ok(channel.to_string())
    }
}

#[cfg(test)]
#[path = "tests/lib.rs"]
mod tests;
