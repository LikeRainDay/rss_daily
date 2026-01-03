use crate::github_trending::{GitHubClient, HistoryManager};
use crate::models::Repository;
use crate::storage::DataStorage;
use anyhow::Result;
use log::info;

pub struct TrendingFetcher {
    client: GitHubClient,
    history: HistoryManager,
    storage: DataStorage,
}

impl TrendingFetcher {
    pub fn new(token: &str) -> Result<Self> {
        Ok(Self {
            client: GitHubClient::new(token)?,
            history: HistoryManager::new()?,
            storage: DataStorage::new("data/github_trending")?,
        })
    }

    /// 拉取并保存每日趋势数据
    pub async fn fetch_daily_trending(
        &mut self,
        languages: &[String],
        min_stars: u32,
    ) -> Result<Vec<Repository>> {
        info!(
            "Fetching daily GitHub trending repositories (min_stars: {})...",
            min_stars
        );

        // 拉取最新数据
        let mut repos = self
            .client
            .fetch_trending_repos(languages, min_stars)
            .await?;
        info!("Fetched {} repositories", repos.len());

        // 数据脱敏：移除可能的敏感信息（如 Discord Token）
        self.sanitize_repos(&mut repos);

        // 保存到数据存储
        let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
        self.storage.save_daily_data(&date, "trending", &repos)?;
        info!("Saved daily trending data for {}", date);

        // 更新历史记录
        self.history.update_history(&repos)?;

        Ok(repos)
    }

    /// 脱敏处理：移除描述和 Readme 中的敏感信息（如 Token）
    fn sanitize_repos(&self, repos: &mut [Repository]) {
        use regex::Regex;
        // 匹配 Discord Token 的常见模式
        // regex crate 默认支持
        let token_regex = Regex::new(
            r"([a-zA-Z0-9]{24}\.[a-zA-Z0-9]{6}\.[a-zA-Z0-9_-]{27}|mfa\.[a-zA-Z0-9_-]{84})",
        )
        .unwrap();

        for repo in repos.iter_mut() {
            if let Some(desc) = &mut repo.description {
                if token_regex.is_match(desc) {
                    log::warn!(
                        "Found potential secret in description of {}, redaction applied.",
                        repo.name
                    );
                    *desc = token_regex
                        .replace_all(desc, "[REDACTED_SECRET]")
                        .to_string();
                }
            }
            if let Some(readme) = &mut repo.readme {
                if token_regex.is_match(readme) {
                    log::warn!(
                        "Found potential secret in README of {}, redaction applied.",
                        repo.name
                    );
                    *readme = token_regex
                        .replace_all(readme, "[REDACTED_SECRET]")
                        .to_string();
                }
            }
        }
    }

    /// 获取历史数据用于排序和去重
    pub fn get_history_data(&self) -> Result<Vec<Repository>> {
        self.history.load_all_history()
    }

    /// 过滤已推荐过的仓库（除非算法允许重新推送）
    pub fn filter_recommended(
        &self,
        repos: &[Repository],
        allow_recommend_again: bool,
    ) -> Vec<Repository> {
        // 如果允许重新推荐，暂时不过滤（让排序算法决定）
        // 这样可以测试功能，实际使用中可以根据需要调整
        if allow_recommend_again {
            log::info!("允许重新推荐模式：不过滤历史记录，由排序算法决定");
            return repos.to_vec();
        }

        // 不允许重新推荐时，过滤所有历史记录
        let history = match self.history.load_all_history() {
            Ok(h) => h,
            Err(e) => {
                log::warn!("Failed to load history: {}", e);
                return repos.to_vec();
            }
        };

        if history.is_empty() {
            return repos.to_vec();
        }

        let recommended_ids: std::collections::HashSet<u64> =
            history.iter().map(|r| r.id).collect();
        let filtered: Vec<_> = repos
            .iter()
            .filter(|repo| !recommended_ids.contains(&repo.id))
            .cloned()
            .collect();

        log::info!(
            "过滤后剩余 {} 个仓库（从 {} 个中过滤）",
            filtered.len(),
            repos.len()
        );
        filtered
    }

    /// 根据算法排序（考虑历史数据）
    pub fn rank_repositories(&self, repos: &mut [Repository]) {
        let history = match self.history.load_all_history() {
            Ok(h) => h,
            Err(_) => {
                // 如果没有历史数据，使用默认排序
                repos.sort_by(|a, b| b.stars.cmp(&a.stars));
                return;
            }
        };

        // 计算综合分数：趋势分数 + 历史表现加分
        repos.sort_by(|a, b| {
            let score_a = self.calculate_rank_score(a, &history);
            let score_b = self.calculate_rank_score(b, &history);
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    fn calculate_rank_score(&self, repo: &Repository, history: &[Repository]) -> f64 {
        // 使用对数降低绝对值影响，让新老项目更公平竞争
        let log_stars = (repo.stars as f64 + 1.0).ln();
        let log_forks = (repo.forks as f64 + 1.0).ln();
        let base_score = log_stars * 3.0 + log_forks * 2.0;

        // 计算增长率分数
        let growth_score = if let Some(old_repo) = history.iter().find(|r| r.id == repo.id) {
            let stars_growth = repo.stars.saturating_sub(old_repo.stars) as f64;
            let growth_rate = stars_growth / (old_repo.stars as f64).max(1.0);

            // 增长率超过 20% 视为显著增长
            if growth_rate > 0.20 {
                growth_rate * 100.0 // 高增长率获得更多分数
            } else if growth_rate > 0.0 {
                growth_rate * 50.0 // 小幅增长也给予奖励
            } else {
                -30.0 // 已推荐但无增长，降低优先级
            }
        } else {
            20.0 // 新仓库获得较高初始加分
        };

        // 时间衰减因子：使用指数衰减
        let days_since_update = (chrono::Utc::now() - repo.updated_at).num_days() as f64;
        let recency_factor = (-days_since_update / 7.0).exp(); // 7天半衰期
        let recency_score = recency_factor * 50.0;

        // 新项目加分（创建时间在30天内）
        let days_since_creation = (chrono::Utc::now() - repo.created_at).num_days() as f64;
        let new_repo_bonus = if days_since_creation <= 30.0 {
            (30.0 - days_since_creation) * 0.5
        } else {
            0.0
        };

        // 综合评分
        let total_score = base_score + growth_score + recency_score + new_repo_bonus;

        log::debug!(
            "Repo: {} | Base: {:.2} | Growth: {:.2} | Recency: {:.2} | New: {:.2} | Total: {:.2}",
            repo.name,
            base_score,
            growth_score,
            recency_score,
            new_repo_bonus,
            total_score
        );

        total_score
    }
}
