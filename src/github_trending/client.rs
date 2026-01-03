use crate::models::{GitHubRepoItem, Repository};
use anyhow::{Context, Result};
use reqwest::{header, Client};
use scraper::{Html, Selector};
use std::time::Duration;

pub struct GitHubClient {
    client: Client,
    token: Option<String>,
}

impl GitHubClient {
    pub fn new(token: &str) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("rss-daily-cursor/1.0")
            .build()?;

        let token = if token.is_empty() {
            log::warn!("⚠️  GitHub Token 未配置，将使用匿名访问（速率限制：60次/小时）");
            log::warn!("   抓取 Trending 需要调用 API 获取详情，强烈建议配置 GITHUB_TOKEN");
            None
        } else {
            Some(token.to_string())
        };

        Ok(Self { client, token })
    }

    /// 抓取 GitHub trending 仓库
    /// 流程:
    /// 1. 抓取 https://github.com/trending/{language}?since=daily 页面
    /// 2. 解析出仓库列表 (Owner/Name)
    /// 3. 并发调用 GitHub API 获取每个仓库的详细信息 (Stars, Forks, Topics, CreatedAt 等)
    /// 4. 并发调用 GitHub API 获取 README 内容
    pub async fn fetch_trending_repos(
        &self,
        languages: &[String],
        min_stars: u32,
    ) -> Result<Vec<Repository>> {
        let mut all_repos = Vec::new();

        for language in languages {
            log::info!("🔍 Scraping trending page for language: {}", language);

            // 1. Scrape trending page to get list of repos
            let trending_repos = match self.scrape_trending_list(language).await {
                Ok(repos) => repos,
                Err(e) => {
                    log::error!("Failed to scrape trending for {}: {}", language, e);
                    continue;
                }
            };

            log::info!(
                "   Found {} repos in trending list for {}",
                trending_repos.len(),
                language
            );

            // 2. Fetch details for each repo
            let mut language_repos = Vec::new();

            // Limit to top items to save API calls/time if strictly needed,
            // but config says "max_items = 15", so maybe we process all scraped (usually 25) and filter later?
            // Let's process all scraped items.
            for (owner, name, stars_today) in trending_repos {
                // Sleep briefly to be nice to API if serial
                // tokio::time::sleep(Duration::from_millis(100)).await;

                match self.fetch_full_repo_info(&owner, &name, stars_today).await {
                    Ok(repo) => {
                        if repo.stars >= min_stars {
                            language_repos.push(repo);
                        }
                    }
                    Err(e) => {
                        log::warn!("   Failed to fetch details for {}/{}: {}", owner, name, e);
                    }
                }
            }

            all_repos.extend(language_repos);
        }

        // 去重 (以防多个语言榜单有重叠)
        all_repos.dedup_by(|a, b| a.id == b.id);

        // 优化排序算法 (Algorithm Optimization)
        // 优先权重: stars_today > stars > forks
        all_repos.sort_by(|a, b| {
            let score_a = Self::calculate_score(a);
            let score_b = Self::calculate_score(b);
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(all_repos)
    }

    /// 计算趋势分数
    fn calculate_score(repo: &Repository) -> f64 {
        // stars_today 是核心指标
        let daily_factor = repo.stars_today.unwrap_or(0) as f64 * 10.0;

        // 总 stars 和 forks 作为辅助指标 (权重较低)
        let total_stars_factor = (repo.stars as f64).log10() * 2.0;
        let forks_factor = (repo.forks as f64).log10() * 1.0;

        // 新颖度加分 (30天内创建的项目给予额外加分)
        let days_since_created = (chrono::Utc::now() - repo.created_at).num_days();
        let freshness_bonus = if days_since_created < 30 {
            (30 - days_since_created) as f64 * 0.5
        } else {
            0.0
        };

        daily_factor + total_stars_factor + forks_factor + freshness_bonus
    }

    /// Scrape the GitHub trending page for a specific language
    /// Returns a list of (owner, repo_name, stars_today) tuples
    async fn scrape_trending_list(
        &self,
        language: &str,
    ) -> Result<Vec<(String, String, Option<u32>)>> {
        let github_lang = self.map_language_to_url_param(language);
        let url = format!("https://github.com/trending/{}?since=daily", github_lang);

        let response = self.client.get(&url).send().await?.text().await?;
        let document = Html::parse_document(&response);
        let row_selector = Selector::parse("article.Box-row").unwrap();
        let title_selector = Selector::parse("h2.h3 a").unwrap();
        let stars_selector = Selector::parse("span.d-inline-block.float-sm-right").unwrap();

        let mut repos = Vec::new();

        for element in document.select(&row_selector) {
            let owner_name = if let Some(link_el) = element.select(&title_selector).next() {
                if let Some(href) = link_el.value().attr("href") {
                    // href should be "/owner/repo"
                    let parts: Vec<&str> = href.trim_start_matches('/').split('/').collect();
                    if parts.len() >= 2 {
                        Some((parts[0].to_string(), parts[1].to_string()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            // Extract "X stars today"
            let stars_today = if let Some(stars_el) = element.select(&stars_selector).next() {
                let text = stars_el.text().collect::<Vec<_>>().join("");
                let text = text.trim();
                // Typically "123 stars today" or "1,234 stars today"
                if let Some(num_str) = text.split_whitespace().next() {
                    let num_clean = num_str.replace(",", "");
                    num_clean.parse::<u32>().ok()
                } else {
                    None
                }
            } else {
                None
            };

            if let Some((owner, name)) = owner_name {
                repos.push((owner, name, stars_today));
            }
        }

        Ok(repos)
    }

    /// Fetch full repository details including README
    async fn fetch_full_repo_info(
        &self,
        owner: &str,
        repo: &str,
        stars_today: Option<u32>,
    ) -> Result<Repository> {
        // 1. Fetch Repo Metadata (API)
        let mut repository = self.fetch_repo_details(owner, repo).await?;

        // Populate scraped data
        repository.stars_today = stars_today;

        // 2. Fetch README (API)
        match self.fetch_readme(owner, repo).await {
            Ok(content) => {
                repository.readme = content;
                // log::info!("   Fetched README for {}/{}", owner, repo);
            }
            Err(e) => {
                log::warn!("   Could not fetch README for {}/{}: {}", owner, repo, e);
            }
        }

        Ok(repository)
    }

    /// Fetch raw README content via GitHub API
    async fn fetch_readme(&self, owner: &str, repo: &str) -> Result<Option<String>> {
        let url = format!("https://api.github.com/repos/{}/{}/readme", owner, repo);

        let mut request = self
            .client
            .get(&url)
            .header("Accept", "application/vnd.github.v3.raw"); // Request raw content

        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !response.status().is_success() {
            // anyhow::bail!("Failed to fetch readme: status {}", response.status());
            return Ok(None); // Treat errors as "no readme" to avoid failing the whole process
        }

        let content = response.text().await?;
        Ok(Some(content))
    }

    /// 获取仓库的详细信息 (API)
    pub async fn fetch_repo_details(&self, owner: &str, repo: &str) -> Result<Repository> {
        let url = format!("https://api.github.com/repos/{}/{}", owner, repo);

        let mut request = self
            .client
            .get(&url)
            .header("Accept", "application/vnd.github.v3+json");

        // 如果有 token，添加认证头
        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .context("Failed to fetch repository details")?;

        // Handle rate limits or errors
        if response.status() == reqwest::StatusCode::FORBIDDEN
            || response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS
        {
            anyhow::bail!("GitHub API Rate Limit Exceeded!");
        }

        if !response.status().is_success() {
            anyhow::bail!("GitHub API Error: {}", response.status());
        }

        let repo_item: GitHubRepoItem = response
            .json()
            .await
            .context("Failed to parse repository details")?;

        Ok(repo_item.into())
    }

    /// 映射语言名称到 GitHub Trending URL 参数
    fn map_language_to_url_param(&self, language: &str) -> String {
        match language.to_lowercase().as_str() {
            "cpp" | "c++" => "c++".to_string(),
            "c#" | "csharp" => "c#".to_string(),
            // Most languages are just lowercased in trending URLs, but special chars need encoding
            // However, scraper/reqwest handles standard URLs.
            // For trending page, typically "c++" works as is in the path.
            "unknown" | "other" => "".to_string(), // All languages
            _ => language.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scrape_trending_list() {
        // Initialize simple client without token
        let client = GitHubClient::new("").unwrap();

        // Test scraping Rust trending
        let result = client.scrape_trending_list("rust").await;

        match result {
            Ok(repos) => {
                println!("Successfully scraped {} rust repos", repos.len());
                for (owner, name, stars) in &repos {
                    println!("- {}/{} (Stars today: {:?})", owner, name, stars);
                }
                assert!(!repos.is_empty(), "Should yield at least one repo");
            }
            Err(e) => panic!("Scraping failed: {}", e),
        }
    }
}
