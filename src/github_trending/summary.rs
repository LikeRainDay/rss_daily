use crate::config::Config;
use crate::models::{Repository, Summary};
use anyhow::{Context, Result};
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

    pub async fn generate_summary(&self, repo: &Repository, language: &str) -> Result<Summary> {
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

    fn generate_chinese_summary(
        &self,
        repo: &Repository,
        description: &str,
    ) -> (String, Vec<String>) {
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

    fn generate_english_summary(
        &self,
        repo: &Repository,
        description: &str,
    ) -> (String, Vec<String>) {
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

    /// 使用 OpenAI 生成总结
    async fn generate_openai_summary(&self, repo: &Repository, language: &str) -> Result<Summary> {
        info!(
            "🤖 Starting OpenAI summary generation for repo: {}",
            repo.name
        );

        let api_key = self.config.summary.api_key.as_ref();
        if api_key.is_none() {
            log::warn!("⚠️  OpenAI API key not configured, falling back to simple summary");
            return Ok(self.generate_simple_summary(repo, language));
        }

        match self.call_openai_api(repo, language, api_key.unwrap()).await {
            Ok(summary) => {
                info!(
                    "✅ Successfully generated AI summary for {}: {} chars",
                    repo.name,
                    summary.content.len()
                );
                Ok(summary)
            }
            Err(e) => {
                log::warn!(
                    "❌ OpenAI API failed for {}: {}. Falling back to simple summary.",
                    repo.name,
                    e
                );
                Ok(self.generate_simple_summary(repo, language))
            }
        }
    }

    /// 调用 OpenAI API
    async fn call_openai_api(
        &self,
        repo: &Repository,
        language: &str,
        api_key: &str,
    ) -> Result<Summary> {
        info!("📡 Fetching README for {}...", repo.name);

        // 获取 README 内容
        let readme = self.fetch_readme(repo).await.unwrap_or_else(|e| {
            log::warn!(
                "⚠️  Failed to fetch README for {}: {}. Using fallback.",
                repo.name,
                e
            );
            "README not available".to_string()
        });

        // 截取 README 前 1000 字符避免 token 超限
        let readme_excerpt = if readme.len() > 1000 {
            format!("{}...", &readme[..1000])
        } else {
            readme
        };

        // 构建 prompt
        let prompt = if language == "zh" {
            format!(
                "请为以下 GitHub 项目生成一个200字以内的简洁总结，重点介绍项目的核心功能、亮点和提供的主要服务。\n\n\
                项目信息:\n\
                名称: {}\n\
                描述: {}\n\
                Stars: {}\n\
                语言: {}\n\
                README摘要:\n{}\n\n\
                要求:\n\
                1. 字数严格控制在200字以内\n\
                2. 突出最有价值的特性和服务内容\n\
                3. 语言简洁专业\n\
                4. 直接输出总结内容，不要额外的格式标记",
                repo.name,
                repo.description.as_deref().unwrap_or("无描述"),
                repo.stars,
                repo.language.as_deref().unwrap_or("未知"),
                readme_excerpt
            )
        } else {
            format!(
                "Generate a concise summary (max 200 characters) for this GitHub project, highlighting core features, key highlights and main services.\n\n\
                Project Info:\n\
                Name: {}\n\
                Description: {}\n\
                Stars: {}\n\
                Language: {}\n\
                README Excerpt:\n{}\n\n\
                Requirements:\n\
                1. Strictly within 200 characters\n\
                2. Highlight most valuable features and services\n\
                3. Professional and concise\n\
                4. Output summary directly without extra formatting",
                repo.name,
                repo.description.as_deref().unwrap_or("No description"),
                repo.stars,
                repo.language.as_deref().unwrap_or("Unknown"),
                readme_excerpt
            )
        };

        // 获取配置
        let base_url = self
            .config
            .summary
            .base_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1");
        let model = self
            .config
            .summary
            .model
            .as_deref()
            .unwrap_or("gpt-4o-mini");

        // 构建请求
        let client = reqwest::Client::new();
        let url = format!("{}/chat/completions", base_url);

        info!(
            "🔧 Preparing OpenAI API request: model={}, base_url={}",
            model, base_url
        );

        let request_body = serde_json::json!({
            "model": model,
            "messages": [{
                "role": "user",
                "content": prompt
            }],
            "temperature": 0.7,
            "max_tokens": 300
        });

        info!("📤 Sending request to OpenAI API...");

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to OpenAI API")?;

        let status = response.status();
        info!("📥 Received response from OpenAI API: status={}", status);

        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            log::error!("❌ OpenAI API error {}: {}", status, error_text);
            anyhow::bail!("OpenAI API returned error {}: {}", status, error_text);
        }

        let result: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse OpenAI API response")?;

        info!("✨ Successfully parsed OpenAI API response");

        // 解析响应
        let content = result["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid response format from OpenAI API"))?
            .trim()
            .to_string();

        info!("📝 Generated summary: {} characters", content.len());

        // 生成 key_points
        let key_points = vec![
            format!("⭐ {} stars", repo.stars),
            format!("💻 {}", repo.language.as_deref().unwrap_or("Unknown")),
            if language == "zh" {
                format!("📅 更新: {}", repo.updated_at.format("%Y-%m-%d"))
            } else {
                format!("📅 Updated: {}", repo.updated_at.format("%Y-%m-%d"))
            },
        ];

        Ok(Summary {
            content,
            language: language.to_string(),
            key_points,
        })
    }

    /// 获取仓库 README 内容
    async fn fetch_readme(&self, repo: &Repository) -> Result<String> {
        let client = reqwest::Client::new();
        let url = format!(
            "https://api.github.com/repos/{}/readme",
            repo.full_name.as_str()
        );

        let response = client
            .get(&url)
            .header(
                "Authorization",
                format!("token {}", self.config.github_token),
            )
            .header("User-Agent", "rss-daily")
            .header("Accept", "application/vnd.github.v3.raw")
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to fetch README: {}", response.status());
        }

        let readme = response.text().await?;
        Ok(readme)
    }

    /// 本地模型总结生成（需要本地模型服务）
    /// 如果失败，不影响生成，回退到简单总结
    async fn generate_local_summary(&self, repo: &Repository, language: &str) -> Result<Summary> {
        // TODO: 实现本地模型调用（如 Ollama、LocalAI 等）
        // 如果失败，回退到简单总结
        match self.call_local_model(repo, language).await {
            Ok(summary) => {
                info!(
                    "Successfully generated local model summary for {}",
                    repo.name
                );
                Ok(summary)
            }
            Err(e) => {
                log::warn!(
                    "Local model call failed for {}: {}, using simple summary",
                    repo.name,
                    e
                );
                Ok(self.generate_simple_summary(repo, language))
            }
        }
    }

    /// 调用本地模型
    async fn call_local_model(&self, _repo: &Repository, _language: &str) -> Result<Summary> {
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
