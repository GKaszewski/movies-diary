use domain::models::DiaryEntry;
use domain::ports::{FeedChannel, RssFeedRenderer};
use rss_feed::extension::atom::{self, AtomExtensionBuilder, Link};
use rss_feed::{ChannelBuilder, GuidBuilder, ItemBuilder};

pub struct RssAdapter {
    base_url: String,
}

impl RssAdapter {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    fn absolute(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

impl RssFeedRenderer for RssAdapter {
    fn render_feed(
        &self,
        entries: &[DiaryEntry],
        channel: &FeedChannel<'_>,
    ) -> Result<String, String> {
        let items = entries
            .iter()
            .map(|e| {
                let item_title = format!(
                    "{} ({})",
                    e.movie().title().value(),
                    e.movie().release_year().value()
                );
                let description = match (e.review().comment(), e.review().spoilers()) {
                    (Some(_), true) => {
                        format!("{}/5 — contains spoilers", e.review().rating().value())
                    }
                    (Some(c), false) => {
                        format!("{}/5 — {}", e.review().rating().value(), c.value())
                    }
                    (None, _) => format!("{}/5", e.review().rating().value()),
                };
                let pub_date = e
                    .review()
                    .watched_at()
                    .and_utc()
                    .format("%a, %d %b %Y %H:%M:%S +0000")
                    .to_string();
                let link = self.absolute(&format!("/movies/{}", e.movie().id().value()));
                let guid = GuidBuilder::default()
                    .value(e.review().id().value().to_string())
                    .permalink(false)
                    .build();
                ItemBuilder::default()
                    .title(Some(item_title))
                    .link(Some(link))
                    .description(Some(description))
                    .pub_date(Some(pub_date))
                    .guid(Some(guid))
                    .build()
            })
            .collect::<Vec<_>>();

        let self_link = Link {
            href: self.absolute(channel.self_path),
            rel: "self".to_string(),
            mime_type: Some("application/rss+xml".to_string()),
            ..Default::default()
        };

        let channel = ChannelBuilder::default()
            .title(channel.title.to_string())
            .link(self.absolute(channel.link_path))
            .description(channel.title.to_string())
            .language(Some("en".to_string()))
            .namespace(("atom".to_string(), atom::NAMESPACE.to_string()))
            .atom_ext(Some(
                AtomExtensionBuilder::default()
                    .links(vec![self_link])
                    .build(),
            ))
            .items(items)
            .build();

        Ok(channel.to_string())
    }
}

#[cfg(test)]
#[path = "tests/lib.rs"]
mod tests;
