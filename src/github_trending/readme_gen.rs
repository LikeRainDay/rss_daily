use super::card::Card;
use crate::locales::get_resources;
use crate::models::Repository;
use anyhow::Result;
use chrono::Utc;
use log::info;
use std::path::Path;

pub struct ReadmeGenerator;

impl ReadmeGenerator {
    pub fn new() -> Self {
        Self
    }

    /// 生成当天的 README
    pub fn generate_daily_readme(
        &self,
        date: &str,
        categories: &[(String, Vec<(Repository, Card)>)], // (category_name, repos_with_cards)
        output_dir: &Path,
        locale: &str, // "en" or "zh"
    ) -> Result<String> {
        let is_cn = locale == "zh";
        let text = get_resources(locale);

        let mut content = String::new();

        // Title with modern styling
        content.push_str(&format!("# 📊 {} - {}\n\n", text.title_prefix, date));
        content.push_str(&format!("> {}\n\n", text.description));

        // === NEW: Overview Section ===
        content.push_str("## 📋 Overview\n\n");

        // Calculate aggregate statistics
        let total_repos: usize = categories.iter().map(|(_, repos)| repos.len()).sum();
        let mut all_repos: Vec<&Repository> = categories
            .iter()
            .flat_map(|(_, repos)| repos.iter().map(|(repo, _)| repo))
            .collect();

        let total_stars: u32 = all_repos.iter().map(|r| r.stars).sum();
        let total_forks: u32 = all_repos.iter().map(|r| r.forks).sum();

        // Get unique languages
        let mut languages: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for repo in &all_repos {
            if let Some(lang) = &repo.language {
                *languages.entry(lang.clone()).or_insert(0) += 1;
            }
        }

        // Top 3 languages
        let mut lang_vec: Vec<_> = languages.iter().collect();
        lang_vec.sort_by(|a, b| b.1.cmp(a.1));
        let top_languages: Vec<String> = lang_vec
            .iter()
            .take(3)
            .map(|(lang, count)| format!("`{}` ({})", lang, count))
            .collect();

        // Display overview stats in a clean, modern format
        content.push_str(&format!(
            "**{}** {} | **{}** ⭐ | **{}** 🍴\n\n",
            total_repos,
            if is_cn { "个项目" } else { "Projects" },
            total_stars,
            total_forks
        ));

        if !top_languages.is_empty() {
            content.push_str(&format!(
                "**{}:** {}\n\n",
                if is_cn {
                    "热门语言"
                } else {
                    "Top Languages"
                },
                top_languages.join(" · ")
            ));
        }

        content.push_str(&format!(
            "**{}:** {}\n\n",
            if is_cn { "更新时间" } else { "Updated" },
            Utc::now().format("%Y-%m-%d %H:%M UTC")
        ));

        // Category distribution
        content.push_str(&format!(
            "**{}:**\n\n",
            if is_cn { "分类分布" } else { "Categories" }
        ));
        for (category_name, repos) in categories {
            if !repos.is_empty() {
                let display_name = self.format_category_name(category_name, locale);
                content.push_str(&format!(
                    "- {} {} ({} {})\n",
                    self.get_category_emoji(category_name),
                    display_name,
                    repos.len(),
                    if is_cn { "项" } else { "items" }
                ));
            }
        }
        content.push_str("\n");

        // Each category
        for (category_name, repos) in categories {
            if repos.is_empty() {
                continue;
            }

            content.push_str("---\n\n");
            let display_name = self.format_category_name(category_name, locale);
            content.push_str(&format!(
                "## {} {}\n\n",
                self.get_category_emoji(category_name),
                display_name
            ));

            // Repository entries
            for (idx, (repo, card)) in repos.iter().enumerate() {
                // Project title
                content.push_str(&format!(
                    "### {}. [{}]({})\n\n",
                    idx + 1,
                    repo.name,
                    repo.html_url
                ));

                // AI Summary as recommendation (推荐理由)
                content.push_str(&format!(
                    "> 🤖 **{}**  \n",
                    if is_cn {
                        "推荐理由"
                    } else {
                        "Why Recommend"
                    }
                ));
                content.push_str(&format!("> *{}*\n\n", card.summary.content));

                // Display key points if available
                if !card.summary.key_points.is_empty() {
                    for point in &card.summary.key_points {
                        content.push_str(&format!("- {}\n", point));
                    }
                    content.push_str("\n");
                }

                // Card image (preserved)
                let image_path = format!(
                    "{}_{}_{}.png",
                    date,
                    category_name,
                    repo.name.replace("/", "_")
                );
                content.push_str(&format!("![{}]({})\n\n", repo.name, image_path));
            }
        }

        // RSS 订阅链接
        content.push_str("---\n\n");
        content.push_str(&format!("## {}\n\n", text.rss_title));
        content.push_str(&format!("{}\n\n", text.rss_desc));

        // Link to daily-top.xml (Relative path from docs/rss/YYYY/MM-DD to docs/rss/daily-top.xml)
        // Path is ../../../daily-top.xml
        content.push_str(&format!(
            "- 🔔 [{}] (../../daily-top.xml)\n",
            text.rss_daily_xml_title
        ));

        // Link to Current Day's Report (Markdown)
        // Path is ../../../GITHUB_TODAY.md or ../../../GITHUB_TODAY_CN.md
        let daily_report_filename = if is_cn {
            "GITHUB_TODAY_CN.md"
        } else {
            "GITHUB_TODAY.md"
        };
        content.push_str(&format!(
            "- 🔔 [{}] (../../{})\n",
            text.rss_daily_report_title, daily_report_filename
        ));

        // Category feeds
        for (category_name, _) in categories {
            let display_name = self.format_category_name(category_name, locale);
            content.push_str(&format!(
                "- 🔔 [{}](../../{}.xml)\n",
                display_name, category_name
            ));
        }

        content.push_str("\n---\n\n");
        content.push_str(&format!(
            "{} {}\n",
            text.footer,
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));

        // File naming logic based on locale
        let filename = if is_cn { "README_CN.md" } else { "README.md" };
        let readme_path = output_dir.join(filename);
        std::fs::write(&readme_path, &content)?;
        info!("Generated {} at {:?}", filename, readme_path);

        // Update GITHUB_TODAY only for default (English) or both?
        let today_filename = if is_cn {
            "GITHUB_TODAY_CN.md"
        } else {
            "GITHUB_TODAY.md"
        };
        let today_path = output_dir.join(today_filename);
        std::fs::write(&today_path, &content)?;
        info!("Generated {}: {:?}", today_filename, today_path);

        Ok(content)
    }

    pub fn generate_landing_readme(&self, output_path: &Path, locale: &str) -> Result<()> {
        let text = get_resources(locale);
        let mut content = String::new();

        // Title & Subtitle
        content.push_str(&format!("# {}\n\n", text.landing_title));
        content.push_str(&format!("> {}\n\n", text.landing_subtitle));

        // Today's Picks Section
        content.push_str(&format!("## {}\n\n", text.landing_today_title));
        content.push_str(&format!("{}\n\n", text.landing_today_desc));

        // Link to Today's Report
        let daily_report_filename = if locale == "zh" {
            "GITHUB_TODAY_CN.md"
        } else {
            "GITHUB_TODAY.md"
        };
        // Link points to docs/rss/GITHUB_TODAY*.md from root
        content.push_str(&format!(
            "**[{}]({})**\n\n",
            text.landing_today_link,
            format!("docs/rss/{}", daily_report_filename)
        ));

        // RSS Subscription
        content.push_str(&format!("## {}\n\n", text.landing_rss_title));
        content.push_str(&format!("{}\n\n", text.landing_rss_desc));
        content.push_str(&format!(
            "- **{}**: [docs/rss/daily-top.xml](docs/rss/daily-top.xml)\n\n",
            text.landing_rss_xml_label
        ));

        // Features
        content.push_str(&format!("## {}\n\n", text.landing_features_title));
        content.push_str(&format!("- {}\n", text.landing_feature_algo));
        content.push_str(&format!("- {}\n", text.landing_feature_daily));
        content.push_str(&format!("- {}\n", text.landing_feature_card));
        content.push_str(&format!("- {}\n\n", text.landing_feature_rss));

        // History
        content.push_str(&format!("## {}\n\n", text.landing_history_title));
        content.push_str(&format!("{}\n", text.landing_history_desc));

        // Write to file
        std::fs::write(output_path, content)?;
        info!("Generated Landing Page: {:?}", output_path);

        Ok(())
    }

    fn get_category_emoji(&self, name: &str) -> &str {
        match name {
            "backend" => "🔧",
            "frontend" => "🎨",
            "mobile" => "📱",
            "ai-ml" => "🤖",
            "daily-top" => "🌟",
            _ => "📦",
        }
    }

    fn format_category_name(&self, name: &str, locale: &str) -> String {
        let text = get_resources(locale);
        match name {
            "backend" => text.cat_backend.to_string(),
            "frontend" => text.cat_frontend.to_string(),
            "mobile" => text.cat_mobile.to_string(),
            "ai-ml" => text.cat_ai_ml.to_string(),
            "daily-top" => text.cat_daily_top.to_string(),
            _ => name.to_string(),
        }
    }
}
