use application::ports::RssFeedRenderer;
use domain::models::DiaryEntry;
use rss_feed::{ChannelBuilder, GuidBuilder, ItemBuilder};

pub struct RssAdapter {
    feed_title: String,
    feed_link: String,
}

impl RssAdapter {
    pub fn new(feed_title: String, feed_link: String) -> Self {
        Self { feed_title, feed_link }
    }
}

impl RssFeedRenderer for RssAdapter {
    fn render_feed(&self, entries: &[DiaryEntry]) -> Result<String, String> {
        let items = entries
            .iter()
            .map(|e| {
                let title = format!(
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
                    .title(Some(title))
                    .description(Some(description))
                    .pub_date(Some(pub_date))
                    .guid(Some(guid))
                    .build()
            })
            .collect::<Vec<_>>();

        let channel = ChannelBuilder::default()
            .title(self.feed_title.clone())
            .link(self.feed_link.clone())
            .description(self.feed_title.clone())
            .items(items)
            .build();

        Ok(channel.to_string())
    }
}
