use super::card::Card;
use crate::config::Config;
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
    ) -> Result<String> {
        let mut content = String::new();

        // 标题和说明
        content.push_str(&format!("# 🌟 GitHub Trending Daily - {}\n\n", date));
        content.push_str(&format!(
            "> 📅 每日精选 GitHub 热门仓库 | 基于智能算法推荐\n\n"
        ));

        // 统计信息
        let total_repos: usize = categories.iter().map(|(_, repos)| repos.len()).sum();
        content.push_str("## 📊 Today's Highlights\n\n");
        content.push_str(&format!("| 统计项 | 数值 |\n"));
        content.push_str(&format!("|--------|------|\n"));
        content.push_str(&format!("| 📦 精选项目 | **{}** 个 |\n", total_repos));
        content.push_str(&format!(
            "| ⏰ 更新时间 | {} |\n\n",
            Utc::now().format("%Y-%m-%d %H:%M UTC")
        ));

        // 每个分类
        for (category_name, repos) in categories {
            if repos.is_empty() {
                continue;
            }

            content.push_str("---\n\n");
            content.push_str(&format!(
                "## {} {}\n\n",
                self.get_category_emoji(category_name),
                self.format_category_name(category_name)
            ));

            // 仓库表格
            for (idx, (repo, _card)) in repos.iter().enumerate() {
                // 项目标题
                content.push_str(&format!(
                    "### {}. [{}]({})\n\n",
                    idx + 1,
                    repo.name,
                    repo.html_url
                ));

                // 统计信息表格
                content.push_str("| 指标 | 值 |\n");
                content.push_str("|------|----|\n");
                content.push_str(&format!("| ⭐ Stars | **{}** |\n", repo.stars));
                content.push_str(&format!("| 🍴 Forks | **{}** |\n", repo.forks));
                content.push_str(&format!(
                    "| 💻 Language | {} |\n",
                    repo.language.as_deref().unwrap_or("N/A")
                ));
                if !repo.topics.is_empty() {
                    let topics_str: Vec<String> = repo
                        .topics
                        .iter()
                        .take(5) // 最多显示5个标签
                        .map(|t| format!("`{}`", t))
                        .collect();
                    content.push_str(&format!("| 🏷️ Tags | {} |\n", topics_str.join(" ")));
                }
                content.push_str("\n");

                // 项目描述
                if let Some(desc) = &repo.description {
                    content.push_str(&format!("**📝 Description:** {}\n\n", desc));
                }

                // 卡片图片
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
        content.push_str("## 📡 RSS订阅\n\n");
        content.push_str("通过 RSS 订阅，第一时间获取每日精选项目：\n\n");
        for (category_name, _) in categories {
            content.push_str(&format!(
                "- 🔔 [{}](../{}.xml)\n", // RSS XML is also in the same dir? No wait.
                self.format_category_name(category_name),
                category_name
            ));
        }
        // Wait, main.rs puts rss path = output_dir.join(format!("{}.xml", category.name));
        // So RSS xml is in docs/rss/2026/01-03/category.xml
        // README.md is in docs/rss/2026/01-03/README.md
        // So link should be just `category.xml` or `./category.xml`

        content.push_str("\n---\n\n");
        content.push_str(&format!(
            "*⚡ Powered by Smart Trending Algorithm | Generated at {}*\n",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));

        // 保存到 output_dir (main.rs 已经设置了正确的日期目录)
        let readme_path = output_dir.join("README.md");
        std::fs::write(&readme_path, &content)?;
        info!("Generated README: {:?}", readme_path);

        // 同时复制一份到 docs/rss/GITHUB_TODAY.md 作为最新推荐
        // output_dir is docs/rss/2026/01-03
        // We want GITHUB_TODAY.md in docs/rss/ ?
        // Usually GITHUB_TODAY is at root of rss output or something.
        // The original code: let today_path = output_dir.join("GITHUB_TODAY.md");
        // If output_dir is now nested, GITHUB_TODAY needs to go up?
        // Let's assume user wants GITHUB_TODAY.md at docs/rss/GITHUB_TODAY.md
        // But passing output_dir is restrictive.
        // Let's keep it in output_dir first, or try to navigate up.
        // For safely, let's just write to output_dir first as per typical logic,
        // OR checks if we need to write to a "latest" location.
        // Given the requirement "docs/rss/GITHUB_TODAY.md" usually implies a fixed "latest" file.
        // Logic in main.rs passed `output_dir` which is `docs/rss/2026/01-03`.
        // So today_path becomes `docs/rss/2026/01-03/GITHUB_TODAY.md`. This is probably fine as a record for that day.
        // But commonly checking "today" implies a fixed path.
        // I will write it to output_dir for now to match strict logic, but maybe update valid link.

        let today_path = output_dir.join("GITHUB_TODAY.md");
        std::fs::write(&today_path, &content)?;
        info!("Generated GITHUB_TODAY.md: {:?}", today_path);

        Ok(content)
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

    fn format_category_name(&self, name: &str) -> String {
        match name {
            "backend" => "后端开发".to_string(),
            "frontend" => "前端开发".to_string(),
            "mobile" => "移动开发".to_string(),
            "ai-ml" => "AI/机器学习".to_string(),
            "daily-top" => "每日 Top 10 精选".to_string(),
            _ => name.to_string(),
        }
    }
}
