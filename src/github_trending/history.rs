use crate::models::Repository;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HistoryEntry {
    repo: Repository,
    recommended_at: chrono::DateTime<chrono::Utc>,
    times_recommended: u32,
}

pub struct HistoryManager {
    history_file: PathBuf,
    history: HashMap<u64, HistoryEntry>,
}

impl HistoryManager {
    pub fn new() -> Result<Self> {
        let history_file = PathBuf::from("data/github_trending/history.json");

        // 确保目录存在
        if let Some(parent) = history_file.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let history = if history_file.exists() {
            let content = std::fs::read_to_string(&history_file)?;
            let entries: Vec<HistoryEntry> = serde_json::from_str(&content).unwrap_or_default();
            entries.into_iter().map(|e| (e.repo.id, e)).collect()
        } else {
            HashMap::new()
        };

        Ok(Self {
            history_file,
            history,
        })
    }

    /// 更新历史记录
    pub fn update_history(&mut self, repos: &[Repository]) -> Result<()> {
        let now = chrono::Utc::now();

        for repo in repos {
            let entry = self.history.entry(repo.id).or_insert_with(|| HistoryEntry {
                repo: repo.clone(),
                recommended_at: now,
                times_recommended: 0,
            });

            entry.repo = repo.clone();
            entry.times_recommended += 1;
            entry.recommended_at = now;
            entry.recommended_at = now;
        }

        self.prune_history();
        self.save()?;
        Ok(())
    }

    /// 清理旧历史记录
    fn prune_history(&mut self) {
        // 1. Remove entries older than 30 days
        let thirty_days_ago = chrono::Utc::now() - chrono::Duration::days(30);
        self.history
            .retain(|_, entry| entry.recommended_at > thirty_days_ago);

        // 2. If still too many (> 1000), keep recent ones
        if self.history.len() > 1000 {
            let mut entries: Vec<_> = self.history.values().cloned().collect();
            // Sort by recommended_at desc
            entries.sort_by(|a, b| b.recommended_at.cmp(&a.recommended_at));

            // Keep top 1000
            let keep_ids: std::collections::HashSet<u64> =
                entries.iter().take(1000).map(|e| e.repo.id).collect();

            self.history.retain(|id, _| keep_ids.contains(id));
        }
    }

    /// 加载所有历史数据
    pub fn load_all_history(&self) -> Result<Vec<Repository>> {
        Ok(self.history.values().map(|e| e.repo.clone()).collect())
    }

    /// 检查仓库是否被推荐过
    pub fn is_recommended(&self, repo_id: u64) -> bool {
        self.history.contains_key(&repo_id)
    }

    /// 获取推荐次数
    pub fn get_recommend_count(&self, repo_id: u64) -> u32 {
        self.history
            .get(&repo_id)
            .map(|e| e.times_recommended)
            .unwrap_or(0)
    }

    /// 保存历史记录
    fn save(&self) -> Result<()> {
        let entries: Vec<HistoryEntry> = self.history.values().cloned().collect();
        let content = serde_json::to_string_pretty(&entries)?;
        std::fs::write(&self.history_file, content)?;
        Ok(())
    }
}
