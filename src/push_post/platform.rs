use crate::models::Repository;
use anyhow::Result;

#[async_trait::async_trait]
pub trait PostPlatform: Send + Sync {
    /// 平台名称
    fn name(&self) -> &str;

    /// 推送单个仓库
    async fn push_repository(&mut self, repo: &Repository, content: &str) -> Result<String>;

    /// 批量推送
    async fn push_batch(&mut self, repos: &[(Repository, String)]) -> Result<Vec<String>> {
        let mut results = Vec::new();
        for (repo, content) in repos {
            match self.push_repository(repo, content).await {
                Ok(id) => results.push(id),
                Err(e) => {
                    log::error!("Failed to push {} to {}: {}", repo.name, self.name(), e);
                }
            }
        }
        Ok(results)
    }
}
