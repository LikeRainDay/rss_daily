mod config;
mod github_trending;
mod models;
mod push_post;
mod storage;

use anyhow::Result;
use log::info;
use std::path::PathBuf;

use config::Config;
use github_trending::{CardGenerator, ReadmeGenerator, RssGenerator, TrendingFetcher};
use push_post::PostPlatform;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    info!("Starting RSS Daily Cursor service...");

    // 加载配置
    let config = Config::load()?;
    info!("Configuration loaded");

    // 初始化组件
    let mut fetcher = TrendingFetcher::new(&config.github_token)?;
    let card_gen = CardGenerator::new(&config);
    let rss_generator = RssGenerator::new();
    let readme_generator = ReadmeGenerator::new();

    // 获取当前日期
    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();

    // 构建输出目录结构: docs/rss/YYYY/MM-DD
    let date_parts: Vec<&str> = date.split('-').collect();
    let year = date_parts.first().unwrap_or(&"2024"); // Default fallback
    let month_day = if date_parts.len() >= 3 {
        format!("{}-{}", date_parts[1], date_parts[2])
    } else {
        date.clone()
    };

    let output_dir = PathBuf::from("docs/rss").join(year).join(&month_day);
    std::fs::create_dir_all(&output_dir)?;
    info!("Output directory set to: {:?}", output_dir);

    // 拉取每日趋势数据（会自动保存到 data 目录）
    info!(
        "Fetching daily GitHub trending repositories (min_stars: {})...",
        config.min_stars
    );
    let mut repos = fetcher
        .fetch_daily_trending(&config.languages, config.min_stars)
        .await?;
    info!("Fetched {} repositories", repos.len());

    // 过滤已推荐过的仓库（除非算法允许重新推送）
    repos = fetcher.filter_recommended(&repos, config.allow_recommend_again);
    info!("After filtering: {} repositories", repos.len());

    // 根据算法排序
    fetcher.rank_repositories(&mut repos);

    // 为每个分类生成 RSS 和卡片
    let mut category_data = Vec::new(); // 用于生成 README

    for category in &config.categories {
        info!("Processing category: {}", category.name);

        // 过滤该分类的仓库
        // 如果 keywords 和 topics 都为空，说明要包含所有仓库（top 10 模式）
        let category_repos: Vec<_> = if category.keywords.is_empty() && category.topics.is_empty() {
            // 不做任何过滤，直接取前 N 个（已经按算法排序）
            repos.iter().take(category.max_items).cloned().collect()
        } else {
            // 传统分类模式：按关键词和主题过滤
            repos
                .iter()
                .filter(|repo| {
                    category
                        .keywords
                        .iter()
                        .any(|keyword| repo.name.to_lowercase().contains(keyword))
                        || category
                            .topics
                            .iter()
                            .any(|topic| repo.topics.contains(topic))
                })
                .take(category.max_items)
                .cloned()
                .collect()
        };

        if category_repos.is_empty() {
            info!("No repositories found for category: {}", category.name);
            continue;
        }

        // Initialize Browser
        let browser_opts = headless_chrome::LaunchOptions::default_builder()
            .window_size(Some((config.image.width + 100, config.image.height + 100)))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build launch options: {}", e))?;
        let browser = headless_chrome::Browser::new(browser_opts)?;

        // 生成卡片和 RSS items (Parallel)
        let mut rss_items = Vec::new();
        let mut cards = Vec::new();

        use futures::StreamExt;

        let results = futures::stream::iter(category_repos.iter().enumerate())
            .map(|(i, repo)| {
                let card_gen = &card_gen;
                let browser = &browser;
                let category = &category;
                let config = &config;
                let output_dir = &output_dir;
                let date = &date;

                async move {
                    info!("Processing repository: {}", repo.name);
                    let result = card_gen
                        .generate_card(
                            repo,
                            &category.language,
                            output_dir,
                            &category.name,
                            config,
                            date,
                            i + 1,
                            browser,
                        )
                        .await;
                    (repo, result)
                }
            })
            .buffer_unordered(5) // Limit concurrency to 5 tabs
            .collect::<Vec<_>>()
            .await;

        for (repo, result) in results {
            match result {
                Ok(card) => {
                    cards.push((repo.clone(), card.clone()));

                    // 创建 RSS item
                    let rss_item = models::RssItem {
                        title: format!(
                            "{} - {}",
                            repo.name,
                            repo.description.as_deref().unwrap_or("")
                        ),
                        link: repo.html_url.clone(),
                        description: card.html.clone(),
                        pub_date: repo.updated_at,
                        image_url: card.image_path.clone(),
                        language: category.language.clone(),
                    };

                    rss_items.push(rss_item);
                }
                Err(e) => {
                    log::warn!("Failed to generate card for {}: {}", repo.name, e);
                    // 即使卡片生成失败，也创建基本的 RSS item
                    let rss_item = models::RssItem {
                        title: format!(
                            "{} - {}",
                            repo.name,
                            repo.description.as_deref().unwrap_or("")
                        ),
                        link: repo.html_url.clone(),
                        description: repo.description.as_deref().unwrap_or("").to_string(),
                        pub_date: repo.updated_at,
                        image_url: String::new(),
                        language: category.language.clone(),
                    };
                    rss_items.push(rss_item);
                }
            }
        }

        // 保存分类数据用于生成 README
        category_data.push((category.name.clone(), cards.clone()));

        // 生成 RSS feed
        let rss_content = rss_generator.generate_feed(
            &category.name,
            &format!(
                "https://your-username.github.io/rss-daily-cursor/rss/{}.xml",
                category.name
            ),
            &rss_items,
        )?;

        // 保存 RSS 文件
        let rss_path = output_dir.join(format!("{}.xml", category.name));
        std::fs::write(&rss_path, rss_content)?;
        info!("Generated RSS feed: {:?}", rss_path);

        // 推送到平台（如果配置了）
        if config.push.enabled {
            info!("Pushing to platforms...");
            for platform_config in &config.push.platforms {
                if let Err(e) = push_to_platform(platform_config, &cards).await {
                    log::error!("Failed to push to {}: {}", platform_config.name, e);
                }
            }
        }
    }

    // 生成当天的 README
    info!("Generating daily README for {}...", date);
    readme_generator.generate_daily_readme(&date, &category_data, &output_dir)?;
    info!("Generated README_{}.md", date);

    info!("RSS generation completed successfully!");
    Ok(())
}

async fn push_to_platform(
    platform_config: &config::PlatformConfig,
    cards: &[(models::Repository, github_trending::card::Card)],
) -> Result<()> {
    match platform_config.name.as_str() {
        "csdn" => {
            let mut platform = push_post::CSDNPlatform::new(
                platform_config.username.clone().unwrap_or_default(),
                platform_config.password.clone().unwrap_or_default(),
            );

            // 准备推送内容
            let items: Vec<_> = cards
                .iter()
                .map(|(repo, card)| (repo.clone(), card.html.clone()))
                .collect();

            // 推送（内部会处理登录）
            platform.push_batch(&items).await?;
            info!("Successfully pushed {} items to CSDN", items.len());
        }
        _ => {
            log::warn!("Unknown platform: {}", platform_config.name);
        }
    }
    Ok(())
}
