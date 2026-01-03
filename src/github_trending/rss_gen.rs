use crate::models::RssItem;
use anyhow::Result;
use chrono::Utc;
use rss::{ChannelBuilder, ItemBuilder};

pub struct RssGenerator;

impl RssGenerator {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_feed(
        &self,
        title: &str,
        link: &str,
        items: &[RssItem],
    ) -> Result<String> {
        let rss_items: Vec<rss::Item> = items
            .iter()
            .map(|item| {
                // 准备描述（包含图片）
                let description = if !item.image_url.is_empty() {
                    format!(
                        r#"<img src="{}" alt="{}" /><br/><br/>{}"#,
                        item.image_url, item.title, item.description
                    )
                } else {
                    item.description.clone()
                };

                ItemBuilder::default()
                    .title(Some(item.title.clone()))
                    .link(Some(item.link.clone()))
                    .pub_date(Some(item.pub_date.to_rfc2822()))
                    .description(Some(description))
                    .build()
            })
            .collect();

        let channel = ChannelBuilder::default()
            .title(format!("{} - GitHub Trending RSS", title))
            .link(link)
            .description(format!("GitHub trending repositories RSS feed for {}", title))
            .language(Some(items.first().map(|i| i.language.as_str()).unwrap_or("en").to_string()))
            .last_build_date(Some(Utc::now().to_rfc2822()))
            .items(rss_items)
            .build();

        Ok(channel.to_string())
    }
}
