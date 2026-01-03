use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub github_token: String,
    pub languages: Vec<String>,
    pub categories: Vec<Category>,
    pub summary: SummaryConfig,
    pub image: ImageConfig,
    pub push: PushConfig,
    #[serde(default = "default_allow_recommend_again")]
    pub allow_recommend_again: bool,
    #[serde(default = "default_min_stars")]
    pub min_stars: u32,  // 最小 stars 数量过滤
}

fn default_allow_recommend_again() -> bool {
    true
}

fn default_min_stars() -> u32 {
    10
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub name: String,
    pub language: String, // "zh" or "en"
    pub keywords: Vec<String>,
    pub topics: Vec<String>,
    pub max_items: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryConfig {
    pub enabled: bool,
    pub provider: String, // "openai", "local", "simple"
    pub api_key: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageConfig {
    pub enabled: bool,
    pub width: u32,
    pub height: u32,
    pub background_color: String,
    pub text_color: String,
    pub font_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushConfig {
    pub enabled: bool,
    pub platforms: Vec<PlatformConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConfig {
    pub name: String, // "csdn", etc.
    pub username: Option<String>,
    pub password: Option<String>,
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, toml::Value>,
}

impl Config {
    pub fn load() -> Result<Self> {
        // 尝试从环境变量加载
        dotenv::dotenv().ok();

        // 从 config.toml 加载
        let config_str = std::fs::read_to_string("config.toml")
            .context("Failed to read config.toml")?;

        let mut config: Config = toml::from_str(&config_str)
            .context("Failed to parse config.toml")?;

        // 环境变量覆盖
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            config.github_token = token;
        }

        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            config.summary.api_key = Some(api_key);
        }

        // 推送平台配置（从环境变量）
        for platform in &mut config.push.platforms {
            match platform.name.as_str() {
                "csdn" => {
                    if let Ok(username) = std::env::var("CSDN_USERNAME") {
                        platform.username = Some(username);
                    }
                    if let Ok(password) = std::env::var("CSDN_PASSWORD") {
                        platform.password = Some(password);
                    }
                }
                _ => {}
            }
        }

        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            github_token: std::env::var("GITHUB_TOKEN").unwrap_or_default(),
            languages: vec!["rust".to_string(), "python".to_string(), "javascript".to_string()],
            categories: vec![
                Category {
                    name: "backend".to_string(),
                    language: "zh".to_string(),
                    keywords: vec!["backend".to_string(), "server".to_string(), "api".to_string()],
                    topics: vec!["backend".to_string(), "api".to_string()],
                    max_items: 20,
                },
                Category {
                    name: "frontend".to_string(),
                    language: "zh".to_string(),
                    keywords: vec!["frontend".to_string(), "ui".to_string(), "react".to_string()],
                    topics: vec!["frontend".to_string(), "ui".to_string()],
                    max_items: 20,
                },
            ],
            summary: SummaryConfig {
                enabled: true,
                provider: "simple".to_string(),
                api_key: None,
                model: None,
            },
            image: ImageConfig {
                enabled: true,
                width: 1200,
                height: 630,
                background_color: "#1a1a1a".to_string(),
                text_color: "#ffffff".to_string(),
                font_size: 24,
            },
            push: PushConfig {
                enabled: false,
                platforms: Vec::new(),
            },
            allow_recommend_again: true,
            min_stars: 10,
        }
    }
}
