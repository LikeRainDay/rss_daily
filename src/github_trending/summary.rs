use crate::models::{Repository, Summary};
use crate::config::Config;
use anyhow::Result;
use log::info;

pub struct SummaryGenerator {
    config: Config,
}

impl SummaryGenerator {
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
        }
    }

    pub async fn generate_summary(
        &self,
        repo: &Repository,
        language: &str,
    ) -> Result<Summary> {
        if !self.config.summary.enabled {
            return Ok(self.generate_simple_summary(repo, language));
        }

        match self.config.summary.provider.as_str() {
            "openai" => self.generate_openai_summary(repo, language).await,
            "local" => self.generate_local_summary(repo, language).await,
            _ => Ok(self.generate_simple_summary(repo, language)),
        }
    }

    /// 简单总结生成（无需 API）
    fn generate_simple_summary(&self, repo: &Repository, language: &str) -> Summary {
        let description = repo.description.as_deref().unwrap_or("No description");

        let (content, key_points) = if language == "zh" {
            self.generate_chinese_summary(repo, description)
        } else {
            self.generate_english_summary(repo, description)
        };

        Summary {
            content,
            language: language.to_string(),
            key_points,
        }
    }

    fn generate_chinese_summary(&self, repo: &Repository, description: &str) -> (String, Vec<String>) {
        let content = format!(
            r#"
## {name}

**项目描述：** {description}

**核心信息：**
- ⭐ Stars: {stars}
- 🍴 Forks: {forks}
- 💻 主要语言: {language}
- 📅 更新时间: {updated_at}
- 🔗 [访问仓库]({url})

**项目亮点：**
{highlights}

**技术栈：** {topics}
"#,
            name = repo.name,
            description = description,
            stars = repo.stars,
            forks = repo.forks,
            language = repo.language.as_deref().unwrap_or("未知"),
            updated_at = repo.updated_at.format("%Y-%m-%d"),
            url = repo.html_url,
            highlights = self.extract_highlights(repo, "zh"),
            topics = if repo.topics.is_empty() {
                "未标注".to_string()
            } else {
                repo.topics.join(", ")
            }
        );

        let key_points = vec![
            format!("⭐ {} stars", repo.stars),
            format!("🍴 {} forks", repo.forks),
            format!("💻 {}", repo.language.as_deref().unwrap_or("未知")),
            format!("📅 最近更新: {}", repo.updated_at.format("%Y-%m-%d")),
        ];

        (content, key_points)
    }

    fn generate_english_summary(&self, repo: &Repository, description: &str) -> (String, Vec<String>) {
        let content = format!(
            r#"
## {name}

**Description:** {description}

**Key Metrics:**
- ⭐ Stars: {stars}
- 🍴 Forks: {forks}
- 💻 Language: {language}
- 📅 Updated: {updated_at}
- 🔗 [View Repository]({url})

**Highlights:**
{highlights}

**Topics:** {topics}
"#,
            name = repo.name,
            description = description,
            stars = repo.stars,
            forks = repo.forks,
            language = repo.language.as_deref().unwrap_or("Unknown"),
            updated_at = repo.updated_at.format("%Y-%m-%d"),
            url = repo.html_url,
            highlights = self.extract_highlights(repo, "en"),
            topics = if repo.topics.is_empty() {
                "Not tagged".to_string()
            } else {
                repo.topics.join(", ")
            }
        );

        let key_points = vec![
            format!("⭐ {} stars", repo.stars),
            format!("🍴 {} forks", repo.forks),
            format!("💻 {}", repo.language.as_deref().unwrap_or("Unknown")),
            format!("📅 Updated: {}", repo.updated_at.format("%Y-%m-%d")),
        ];

        (content, key_points)
    }

    fn extract_highlights(&self, repo: &Repository, language: &str) -> String {
        let mut highlights = Vec::new();

        if repo.stars > 1000 {
            highlights.push(if language == "zh" {
                "🔥 热门项目（超过 1000 stars）".to_string()
            } else {
                "🔥 Popular project (1000+ stars)".to_string()
            });
        }

        if repo.forks > 100 {
            highlights.push(if language == "zh" {
                "📦 活跃维护（超过 100 forks）".to_string()
            } else {
                "📦 Actively maintained (100+ forks)".to_string()
            });
        }

        let days_since_update = (chrono::Utc::now() - repo.updated_at).num_days();
        if days_since_update <= 7 {
            highlights.push(if language == "zh" {
                "✨ 最近更新（7天内）".to_string()
            } else {
                "✨ Recently updated (within 7 days)".to_string()
            });
        }

        if highlights.is_empty() {
            if language == "zh" {
                "新兴项目，值得关注".to_string()
            } else {
                "Emerging project worth watching".to_string()
            }
        } else {
            highlights.join("\n")
        }
    }

    /// OpenAI API 总结生成（需要配置 API key）
    /// 如果失败，不影响生成，回退到简单总结
    async fn generate_openai_summary(
        &self,
        repo: &Repository,
        language: &str,
    ) -> Result<Summary> {
        // 检查是否有 API key
        let api_key = match &self.config.summary.api_key {
            Some(key) if !key.is_empty() => key,
            _ => {
                info!("OpenAI API key not configured, using simple summary");
                return Ok(self.generate_simple_summary(repo, language));
            }
        };

        // 尝试调用 OpenAI API（如果失败，回退到简单总结）
        match self.call_openai_api(repo, language, api_key).await {
            Ok(summary) => {
                info!("Successfully generated OpenAI summary for {}", repo.name);
                Ok(summary)
            }
            Err(e) => {
                log::warn!("OpenAI API call failed for {}: {}, using simple summary", repo.name, e);
                Ok(self.generate_simple_summary(repo, language))
            }
        }
    }

    /// 调用 OpenAI API
    async fn call_openai_api(
        &self,
        repo: &Repository,
        language: &str,
        _api_key: &str,
    ) -> Result<Summary> {
        // TODO: 实现实际的 OpenAI API 调用
        // 这里是一个示例结构，实际需要根据 OpenAI API 文档实现

        let _prompt = if language == "zh" {
            format!(
                "请为以下 GitHub 仓库生成一个简洁的中文总结和推荐理由：\n\n\
                仓库名称：{}\n\
                描述：{}\n\
                Stars：{}\n\
                语言：{}\n\
                主题：{}\n\n\
                请提供：1. 项目总结 2. 推荐理由 3. 关键特点",
                repo.name,
                repo.description.as_deref().unwrap_or("无描述"),
                repo.stars,
                repo.language.as_deref().unwrap_or("未知"),
                repo.topics.join(", ")
            )
        } else {
            format!(
                "Please generate a concise English summary and recommendation reason for this GitHub repository:\n\n\
                Name: {}\n\
                Description: {}\n\
                Stars: {}\n\
                Language: {}\n\
                Topics: {}\n\n\
                Please provide: 1. Project summary 2. Recommendation reason 3. Key features",
                repo.name,
                repo.description.as_deref().unwrap_or("No description"),
                repo.stars,
                repo.language.as_deref().unwrap_or("Unknown"),
                repo.topics.join(", ")
            )
        };

        // 实际实现需要使用 reqwest 调用 OpenAI API
        // 示例：
        // let client = reqwest::Client::new();
        // let response = client
        //     .post("https://api.openai.com/v1/chat/completions")
        //     .header("Authorization", format!("Bearer {}", api_key))
        //     .json(&json!({
        //         "model": self.config.summary.model.as_deref().unwrap_or("gpt-3.5-turbo"),
        //         "messages": [{"role": "user", "content": prompt}]
        //     }))
        //     .send()
        //     .await?;
        //
        // let result: serde_json::Value = response.json().await?;
        // // 解析结果并生成 Summary

        // 暂时返回错误，触发回退到简单总结
        anyhow::bail!("OpenAI API not fully implemented yet")
    }

    /// 本地模型总结生成（需要本地模型服务）
    /// 如果失败，不影响生成，回退到简单总结
    async fn generate_local_summary(
        &self,
        repo: &Repository,
        language: &str,
    ) -> Result<Summary> {
        // TODO: 实现本地模型调用（如 Ollama、LocalAI 等）
        // 如果失败，回退到简单总结
        match self.call_local_model(repo, language).await {
            Ok(summary) => {
                info!("Successfully generated local model summary for {}", repo.name);
                Ok(summary)
            }
            Err(e) => {
                log::warn!("Local model call failed for {}: {}, using simple summary", repo.name, e);
                Ok(self.generate_simple_summary(repo, language))
            }
        }
    }

    /// 调用本地模型
    async fn call_local_model(
        &self,
        _repo: &Repository,
        _language: &str,
    ) -> Result<Summary> {
        // TODO: 实现本地模型调用
        // 示例：调用 Ollama API
        // let client = reqwest::Client::new();
        // let response = client
        //     .post("http://localhost:11434/api/generate")
        //     .json(&json!({
        //         "model": "llama2",
        //         "prompt": format!("Summarize this GitHub repo: {}", repo.name)
        //     }))
        //     .send()
        //     .await?;

        anyhow::bail!("Local model API not implemented yet")
    }
}
