use crate::models::Repository;
use crate::push_post::PostPlatform;
use anyhow::{Result, Context};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct CSDNPlatform {
    client: Client,
    username: String,
    password: String,
    cookie: Option<String>,
}

#[derive(Debug, Serialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    code: i32,
    message: String,
    data: Option<LoginData>,
}

#[derive(Debug, Deserialize)]
struct LoginData {
    token: Option<String>,
}

#[derive(Debug, Serialize)]
struct ArticleRequest {
    title: String,
    content: String,
    markdowncontent: String,
    tags: String,
    #[serde(rename = "type")]
    article_type: String,
    status: String,
}

impl CSDNPlatform {
    pub fn new(username: String, password: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("rss-daily-cursor/1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            username,
            password,
            cookie: None,
        }
    }

    /// 登录获取 Cookie（公开方法）
    pub async fn login(&mut self) -> Result<()> {
        let login_url = "https://passport.csdn.net/v1/api/login";

        let request = LoginRequest {
            username: self.username.clone(),
            password: self.password.clone(),
        };

        let response = self
            .client
            .post(login_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send login request")?;

        // 提取 Cookie
        if let Some(cookie_header) = response.headers().get("set-cookie") {
            self.cookie = Some(cookie_header.to_str()?.to_string());
        }

        let login_resp: LoginResponse = response
            .json()
            .await
            .context("Failed to parse login response")?;

        if login_resp.code != 200 {
            anyhow::bail!("CSDN login failed: {}", login_resp.message);
        }

        Ok(())
    }

    /// 发布文章
    async fn publish_article(&self, title: &str, content: &str, tags: &str) -> Result<String> {
        if self.cookie.is_none() {
            anyhow::bail!("Not logged in. Call login() first.");
        }

        let publish_url = "https://editor.csdn.net/md?not_checkout=1";

        let article = ArticleRequest {
            title: title.to_string(),
            content: content.to_string(),
            markdowncontent: content.to_string(),
            tags: tags.to_string(),
            article_type: "original".to_string(),
            status: "2".to_string(), // 2 = 发布
        };

        let response = self
            .client
            .post(publish_url)
            .header("Cookie", self.cookie.as_ref().unwrap())
            .json(&article)
            .send()
            .await
            .context("Failed to publish article")?;

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to publish article: {}", text);
        }

        // 返回文章 ID（实际需要从响应中解析）
        Ok("article_id".to_string())
    }
}

#[async_trait::async_trait]
impl PostPlatform for CSDNPlatform {
    fn name(&self) -> &str {
        "CSDN"
    }

    async fn push_repository(&mut self, repo: &Repository, content: &str) -> Result<String> {
        // 确保已登录
        if self.cookie.is_none() {
            self.login().await?;
        }

        let title = format!("GitHub 推荐：{}", repo.name);
        let tags = format!("GitHub,{},{}",
            repo.language.as_deref().unwrap_or("编程"),
            repo.topics.first().unwrap_or(&"开源".to_string())
        );

        self.publish_article(&title, content, &tags).await
    }
}
